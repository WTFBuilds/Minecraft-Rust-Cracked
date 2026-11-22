//! Client network session.
//!
//! Runs on a background thread with its own tokio runtime. The GUI reads
//! [`ClientState`] each frame (world + dirty sections), and pushes block
//! edits through an unbounded channel — GUI and network never share locks
//! across awaits.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use futures::channel::mpsc::UnboundedReceiver;
use futures::{SinkExt, StreamExt};
use melon_proto::{
    ChunkData, ClientMessage, ClientSessionCodec, Handshake, PlayerEntry, ServerMessage, WorldInfo,
    PROTOCOL_VERSION,
};
use melon_world::{block, raycast, Chunk, World};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use crate::mesh::{self, Vertex};

/// A remote player as last reported by the server.
#[derive(Debug, Clone)]
pub struct RemotePlayer {
    pub name: String,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub held: u8,
    /// Walk-swing phase, advanced by the distance moved between updates.
    pub swing_phase: f32,
    pub swing_amp: f32,
}

/// The hotbar palette: (block id, display name) per slot.
pub const HOTBAR: [(u16, &str); 9] = [
    (1, "grass"),
    (2, "dirt"),
    (3, "stone"),
    (4, "red wool"),
    (5, "blue wool"),
    (6, "yellow wool"),
    (7, "green wool"),
    (8, "iron"),
    (9, "gold"),
];

/// GPU-facing player updates, drained by the GUI each frame.
#[derive(Debug, Clone, Copy)]
pub enum PlayerEvent {
    Add(u64),
    Remove(u64),
    Move(u64),
    Held(u64),
}

/// Shared between the GUI thread and the network thread.
pub struct ClientState {
    pub world: World,
    dirty: HashSet<(i32, i32, i32)>,
    /// Spawn from `WorldInfo`, taken once by the GUI camera.
    spawn: Option<[f32; 3]>,
    pub server_name: String,
    pub motd: String,
    /// Our own player id (from the handshake reply) — filters our own
    /// movement/chat broadcasts.
    pub self_id: Option<u64>,
    /// Remote players by id.
    pub players: std::collections::HashMap<u64, RemotePlayer>,
    /// Chat log (formatted lines, oldest first, capped).
    pub chat_lines: std::collections::VecDeque<String>,
    /// `true` once every streamed chunk has arrived.
    pub ready: bool,
    pub connected: bool,
    /// Set on auth rejection or connection loss — the GUI exits on it.
    pub error: Option<String>,
    pub received_chunks: u32,
    expected_chunks: u32,
    pub blocks_changed: u32,
    player_events: Vec<PlayerEvent>,
}

impl ClientState {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            world: World::new(),
            dirty: HashSet::new(),
            spawn: None,
            server_name: String::new(),
            motd: String::new(),
            self_id: None,
            players: std::collections::HashMap::new(),
            chat_lines: std::collections::VecDeque::new(),
            ready: false,
            connected: false,
            error: None,
            received_chunks: 0,
            expected_chunks: 0,
            blocks_changed: 0,
            player_events: Vec::new(),
        }))
    }

    /// Drain the dirty-section set (world coords → section keys).
    pub fn take_dirty(&mut self) -> Vec<(i32, i32, i32)> {
        self.dirty.drain().collect()
    }

    /// One-shot spawn handoff to the camera.
    pub fn take_spawn(&mut self) -> Option<[f32; 3]> {
        self.spawn.take()
    }

    /// Drain GPU-facing player updates.
    pub fn take_player_events(&mut self) -> Vec<PlayerEvent> {
        std::mem::take(&mut self.player_events)
    }

    /// Push a formatted chat line (capped — the GUI shows the tail).
    fn push_chat(&mut self, line: String) {
        self.chat_lines.push_back(line);
        while self.chat_lines.len() > 100 {
            self.chat_lines.pop_front();
        }
    }

    /// Mesh a section from the current world (GUI calls while holding the
    /// lock; meshing is fast and lock time stays tiny).
    pub fn mesh_section(&self, key: (i32, i32, i32)) -> Vec<Vertex> {
        mesh::mesh_section(&self.world, key.0, key.1, key.2)
    }

    /// Pick a block edit under the crosshair: `place` places stone on the
    /// hit face, breaking sets the hit block to air.
    pub fn pick_edit(
        &self,
        origin: [f32; 3],
        dir: [f32; 3],
        place: bool,
        place_block: u16,
    ) -> Option<ClientMessage> {
        let hit = raycast(&self.world, origin, dir, 6.0)?;
        if place {
            let [x, y, z] = hit.neighbor();
            if self.world.get_block(x, y, z).unwrap_or(block::AIR) == block::AIR {
                Some(ClientMessage::SetBlock {
                    x,
                    y,
                    z,
                    block: place_block,
                })
            } else {
                None
            }
        } else {
            let [x, y, z] = hit.block;
            if self.world.get_block(x, y, z).unwrap_or(block::AIR) != block::AIR {
                Some(ClientMessage::SetBlock {
                    x,
                    y,
                    z,
                    block: block::AIR,
                })
            } else {
                None
            }
        }
    }

    fn mark_block_dirty(&mut self, x: i32, y: i32, z: i32) {
        let (sx, sy, sz) = (x.div_euclid(16), y.div_euclid(16), z.div_euclid(16));
        let (lx, ly, lz) = (x.rem_euclid(16), y.rem_euclid(16), z.rem_euclid(16));
        self.dirty.insert((sx, sy, sz));
        // Boundary blocks change faces in the neighboring section too.
        if lx == 0 {
            self.dirty.insert((sx - 1, sy, sz));
        }
        if lx == 15 {
            self.dirty.insert((sx + 1, sy, sz));
        }
        if ly == 0 {
            self.dirty.insert((sx, sy - 1, sz));
        }
        if ly == 15 {
            self.dirty.insert((sx, sy + 1, sz));
        }
        if lz == 0 {
            self.dirty.insert((sx, sy, sz - 1));
        }
        if lz == 15 {
            self.dirty.insert((sx, sy, sz + 1));
        }
    }

    fn mark_chunk_dirty(&mut self, cx: i32, cz: i32) {
        if let Some(chunk) = self.world.chunk(cx, cz) {
            for sy in 0..16usize {
                if !chunk.section(sy).is_empty() {
                    self.dirty.insert((cx, sy as i32, cz));
                }
            }
        }
        // Neighboring loaded chunks re-mesh: their boundary faces change.
        for (ncx, ncz) in [(cx - 1, cz), (cx + 1, cz), (cx, cz - 1), (cx, cz + 1)] {
            if let Some(chunk) = self.world.chunk(ncx, ncz) {
                for sy in 0..16usize {
                    if !chunk.section(sy).is_empty() {
                        self.dirty.insert((ncx, sy as i32, ncz));
                    }
                }
            }
        }
    }

    fn handle_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::HandshakeResult {
                accepted,
                player_id,
                server_name,
                motd,
                rejection,
                ..
            } => {
                if accepted {
                    self.self_id = Some(player_id);
                    self.server_name = server_name;
                    self.motd = motd;
                } else {
                    self.error = Some(format!("rejected by server: {rejection:?}"));
                }
            }
            ServerMessage::WorldInfo(info) => {
                let WorldInfo { spawn, chunks } = info;
                self.spawn = Some(spawn);
                self.expected_chunks = chunks;
                if chunks == 0 {
                    self.ready = true;
                }
            }
            ServerMessage::ChunkData(ChunkData {
                cx,
                cz,
                section_mask,
                sections,
            }) => match Chunk::decode_sections(section_mask, &sections) {
                Ok(chunk) => {
                    self.world.insert(cx, cz, chunk);
                    self.mark_chunk_dirty(cx, cz);
                    self.received_chunks += 1;
                    if self.received_chunks >= self.expected_chunks && self.expected_chunks > 0 {
                        self.ready = true;
                    }
                }
                Err(err) => {
                    self.error = Some(format!("bad chunk ({cx},{cz}): {err}"));
                }
            },
            ServerMessage::BlockUpdate { x, y, z, block } => {
                self.world.set_block(x, y, z, block);
                self.mark_block_dirty(x, y, z);
                self.blocks_changed += 1;
            }
            ServerMessage::PlayerList { players } => {
                for PlayerEntry { id, name } in players {
                    let name_for_new = name.clone();
                    let player = self.players.entry(id).or_insert_with(|| {
                        self.player_events.push(PlayerEvent::Add(id));
                        RemotePlayer {
                            name: name_for_new,
                            pos: [8.0, 6.0, 8.0],
                            yaw: 0.0,
                            pitch: 0.0,
                            held: 0,
                            swing_phase: 0.0,
                            swing_amp: 0.0,
                        }
                    });
                    player.name = name;
                }
            }
            ServerMessage::PlayerJoin { id, name } => {
                if Some(id) != self.self_id && !self.players.contains_key(&id) {
                    self.players.insert(
                        id,
                        RemotePlayer {
                            name: name.clone(),
                            pos: [8.0, 6.0, 8.0],
                            yaw: 0.0,
                            pitch: 0.0,
                            held: 0,
                            swing_phase: 0.0,
                            swing_amp: 0.0,
                        },
                    );
                    self.player_events.push(PlayerEvent::Add(id));
                }
                self.push_chat(format!("▸ {name} joined"));
            }
            ServerMessage::PlayerLeave { id } => {
                if let Some(player) = self.players.remove(&id) {
                    self.player_events.push(PlayerEvent::Remove(id));
                    self.push_chat(format!("▸ {} left", player.name));
                }
            }
            ServerMessage::PlayerMove {
                id,
                pos,
                yaw,
                pitch,
            } => {
                if Some(id) == self.self_id {
                    return; // our own broadcast echo — already local
                }
                if let Some(player) = self.players.get_mut(&id) {
                    player.pos = pos;
                    player.yaw = yaw;
                    player.pitch = pitch;
                    self.player_events.push(PlayerEvent::Move(id));
                }
            }
            ServerMessage::PlayerHeld { id, slot } => {
                if let Some(player) = self.players.get_mut(&id) {
                    player.held = slot;
                    self.player_events.push(PlayerEvent::Held(id));
                }
            }
            ServerMessage::Chat { id, name, text } => {
                self.push_chat(format!("{name}: {text}"));
                let _ = id;
            }
            ServerMessage::Pong { .. } => {}
        }
    }
}

/// Blocking connect + handshake. Fills `state` with the auth result.
/// Returns the live framed connection for [`spawn_session_thread`].
pub async fn connect_and_auth(
    host: &str,
    port: u16,
    username: &str,
    state: &Mutex<ClientState>,
) -> Result<Framed<TcpStream, ClientSessionCodec>, String> {
    let tcp = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        TcpStream::connect((host, port)),
    )
    .await
    .map_err(|_| format!("connect {host}:{port} timed out (is the server running?)"))?
    .map_err(|e| format!("connect {host}:{port} failed: {e}"))?;
    let mut framed = Framed::new(tcp, ClientSessionCodec::new());

    framed
        .send(ClientMessage::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            client_build: format!("melon-client {}", env!("CARGO_PKG_VERSION")),
            username: username.to_string(),
        }))
        .await
        .map_err(|e| format!("send handshake: {e}"))?;

    let reply = framed
        .next()
        .await
        .ok_or("server closed before the auth result")?
        .map_err(|e| format!("auth protocol error: {e}"))?;

    if let ServerMessage::HandshakeResult {
        accepted,
        rejection,
        ..
    } = &reply
    {
        if !accepted {
            return Err(format!("server rejected us: {rejection:?}"));
        }
    } else {
        return Err(format!("expected handshake result, got {reply:?}"));
    }
    state.lock().unwrap().handle_message(reply);
    Ok(framed)
}

/// The session loop body (async; runs on the network thread's runtime).
async fn session(
    framed: Framed<TcpStream, ClientSessionCodec>,
    state: Arc<Mutex<ClientState>>,
    mut edit_rx: UnboundedReceiver<ClientMessage>,
) -> Result<(), String> {
    state.lock().unwrap().connected = true;
    let (mut sink, mut stream) = framed.split();

    loop {
        tokio::select! {
            wire = stream.next() => {
                match wire {
                    Some(Ok(msg)) => {
                        state.lock().unwrap().handle_message(msg);
                        if let Some(err) = state.lock().unwrap().error.clone() {
                            return Err(err);
                        }
                    }
                    Some(Err(err)) => return Err(format!("protocol error: {err}")),
                    None => return Err("server closed the connection".to_string()),
                }
            }
            edit = edit_rx.next() => {
                match edit {
                    Some(msg) => {
                        sink.send(msg).await.map_err(|e| format!("send edit: {e}"))?;
                    }
                    None => return Err("window closed".to_string()),
                }
            }
        }
    }
}

/// Spawn the network thread (own tokio runtime — the TCP connection must
/// live on the *same* runtime that created it, so connect+auth+session all
/// happen here). The auth outcome goes back through `auth_tx`: the GUI
/// fails fast on rejection and opens the window only after a success.
///
/// The GUI exits the process on close, which takes the thread with it.
pub fn spawn_session_thread(
    host: String,
    port: u16,
    name: String,
    state: Arc<Mutex<ClientState>>,
    edit_rx: UnboundedReceiver<ClientMessage>,
    auth_tx: std::sync::mpsc::Sender<Result<(), String>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("network runtime");
        let session_state = Arc::clone(&state);
        let result = runtime.block_on(async move {
            let framed = match connect_and_auth(&host, port, &name, &session_state).await {
                Ok(framed) => {
                    let _ = auth_tx.send(Ok(()));
                    framed
                }
                Err(err) => {
                    let _ = auth_tx.send(Err(err.clone()));
                    return Err(err);
                }
            };
            session(framed, session_state, edit_rx).await
        });
        {
            let mut st = state.lock().unwrap();
            st.connected = false;
            if let Err(err) = &result {
                if st.error.is_none() {
                    st.error = Some(err.clone());
                }
            }
        }
        if let Err(err) = result {
            eprintln!("network session ended: {err}");
        }
    })
}
