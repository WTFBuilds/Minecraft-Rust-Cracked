//! GUI mode — Phase 4: connected world with presence + chat.
//!
//! Remote players render as colored boxes; joins/leaves/moves flow through
//! the protocol; chat is an egui bottom panel (T to type, Enter to send).
//! Input model unchanged: grabbed cursor, recenter-to-middle mouse-look,
//! WASD/Space/Shift fly — all suppressed while the chat field has focus.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use futures::channel::mpsc::UnboundedSender;
use glam::{Mat4, Vec3};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

use crate::camera::{Camera, SPAWN_PITCH, SPAWN_YAW};
use crate::net::{spawn_session_thread, ClientState};
use crate::renderer::Gpu;

/// How often the client reports its position (Phase 4: client-authoritative
/// movement, server-relayed; Phase 6 adds clamps).
const MOVE_SEND_INTERVAL: f32 = 0.1;
/// How much of the chat log to show at once.
const CHAT_TAIL: usize = 10;

pub fn run(host: &str, port: u16, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let state = ClientState::new();
    let (edit_tx, edit_rx) = futures::channel::mpsc::unbounded::<melon_proto::ClientMessage>();

    // Connect + auth happen on the network thread (the TCP connection must
    // live on the runtime that owns it). The GUI waits for the auth outcome
    // and fails fast before opening any window.
    let (auth_tx, auth_rx) = std::sync::mpsc::channel();
    spawn_session_thread(
        host.to_string(),
        port,
        name.to_string(),
        Arc::clone(&state),
        edit_rx,
        auth_tx,
    );
    auth_rx.recv().map_err(|_| "network thread died")??;
    {
        // NOTE: one lock scope — two `state.lock()` temporaries in a single
        // `println!` deadlock against each other.
        let st = state.lock().expect("state");
        println!("connected — {} · {}", st.server_name, st.motd);
    }

    let event_loop = EventLoop::new()?;
    let mut app = App {
        state,
        edit_tx,
        gpu: None,
        camera: None,
        player: None,
        third_person: false,
        keys: HashSet::new(),
        last_frame: None,
        mouse_captured: false,
        skip_warp_event: false,
        egui_state: None,
        selected_slot: 0,
        swing_phase: 0.0,
        swing_amp: 0.0,
        prev_feet: None,
        chat_input: String::new(),
        chat_focused: false,
        chat_focus_requested: false,
        move_timer: 0.0,
        exited: false,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

/// Local player simulation: feet position + velocity, Minecraft-style.
struct PlayerSim {
    feet: [f32; 3],
    vel: [f32; 3],
    on_ground: bool,
    /// Creative-style flight (F): no gravity, no collision.
    flying: bool,
}

/// Per-frame movement intent (from the keyboard, in `[−1, 1]` ranges).
#[derive(Debug, Clone, Copy, Default)]
pub struct MoveInput {
    /// W/S — forward/back.
    pub forward: f32,
    /// D/A — right/left.
    pub strafe: f32,
    /// Space/Shift — up/down.
    pub vertical: f32,
}

/// UI swatch color per block id (matches the atlas tiles).
fn block_color(block: u16) -> egui::Color32 {
    use melon_world::block;
    match block {
        block::GRASS => egui::Color32::from_rgb(106, 170, 64),
        block::DIRT => egui::Color32::from_rgb(134, 96, 67),
        block::STONE => egui::Color32::from_rgb(127, 127, 127),
        block::WOOL_RED => egui::Color32::from_rgb(176, 46, 38),
        block::WOOL_BLUE => egui::Color32::from_rgb(53, 57, 157),
        block::WOOL_YELLOW => egui::Color32::from_rgb(248, 197, 39),
        block::WOOL_GREEN => egui::Color32::from_rgb(93, 124, 22),
        block::IRON_BLOCK => egui::Color32::from_rgb(216, 216, 216),
        block::GOLD_BLOCK => egui::Color32::from_rgb(250, 214, 53),
        _ => egui::Color32::GRAY,
    }
}

struct App {
    state: Arc<Mutex<ClientState>>,
    /// Local physics state — `Some` once the server spawn arrives.
    player: Option<PlayerSim>,
    /// F5 camera: first-person (default) vs third-person (body visible).
    third_person: bool,
    edit_tx: UnboundedSender<melon_proto::ClientMessage>,
    gpu: Option<Gpu>,
    camera: Option<Camera>,
    keys: HashSet<PhysicalKey>,
    last_frame: Option<Instant>,
    mouse_captured: bool,
    /// The next `CursorMoved` is our own `set_cursor_position` warp — skip it.
    skip_warp_event: bool,
    egui_state: Option<egui_winit::State>,
    selected_slot: usize,
    swing_phase: f32,
    swing_amp: f32,
    prev_feet: Option<[f32; 3]>,
    chat_input: String,
    chat_focused: bool,
    chat_focus_requested: bool,
    move_timer: f32,
    /// Set after the network session reports failure — exits next frame.
    exited: bool,
}

impl App {
    fn move_input(&self) -> MoveInput {
        if self.chat_focused {
            return MoveInput::default(); // keyboard belongs to the chat field
        }
        let pressed = |code: KeyCode| self.keys.contains(&PhysicalKey::Code(code));
        MoveInput {
            forward: (pressed(KeyCode::KeyW) as i32 - pressed(KeyCode::KeyS) as i32) as f32,
            strafe: (pressed(KeyCode::KeyD) as i32 - pressed(KeyCode::KeyA) as i32) as f32,
            vertical: (pressed(KeyCode::Space) as i32 - pressed(KeyCode::ShiftLeft) as i32) as f32,
        }
    }

    fn grab_cursor(&mut self, window: &Window) {
        // Locked on platforms that support it, Confined as fallback.
        let grabbed = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
            .is_ok();
        window.set_cursor_visible(!grabbed);
        self.mouse_captured = grabbed;
        self.skip_warp_event = false;
    }

    fn release_cursor(&mut self, window: &Window) {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
        self.mouse_captured = false;
    }

    /// Pull network-driven world updates into GPU meshes.
    fn sync_world(&mut self) {
        let state = Arc::clone(&self.state);
        let spawn = state.lock().expect("state").take_spawn();
        if let Some([x, y, z]) = spawn {
            self.player = Some(PlayerSim {
                feet: [x, y, z],
                vel: [0.0, 0.0, 0.0],
                on_ground: false,
                flying: false,
            });
            self.camera = Some(Camera::new(
                Vec3::new(x, y + melon_world::PLAYER_EYE_HEIGHT, z),
                SPAWN_YAW,
                SPAWN_PITCH,
            ));
        }
        let dirty = state.lock().expect("state").take_dirty();
        if dirty.is_empty() {
            return;
        }
        let mut remesh = Vec::with_capacity(dirty.len());
        {
            let st = state.lock().expect("state");
            for key in &dirty {
                remesh.push((*key, st.mesh_section(*key)));
            }
        }
        let Some(gpu) = self.gpu.as_mut() else { return };
        for (key, vertices) in remesh {
            gpu.update_section(key, vertices);
        }
    }

    /// Pull player join/leave/move events into GPU player boxes.
    fn sync_players(&mut self) {
        let state = Arc::clone(&self.state);
        let events = state.lock().expect("state").take_player_events();
        if events.is_empty() {
            return;
        }
        let Some(gpu) = self.gpu.as_mut() else { return };
        for event in events {
            match event {
                crate::net::PlayerEvent::Add(id)
                | crate::net::PlayerEvent::Move(id)
                | crate::net::PlayerEvent::Held(id) => {
                    let (pos, yaw, shirt, held, swing) = {
                        let st = state.lock().expect("state");
                        let Some(player) = st.players.get(&id) else {
                            continue;
                        };
                        // Stable shirt color per player.
                        let shirt = [
                            crate::atlas::tile::SHIRT,
                            crate::atlas::tile::WOOL_RED,
                            crate::atlas::tile::WOOL_BLUE,
                            crate::atlas::tile::WOOL_YELLOW,
                            crate::atlas::tile::WOOL_GREEN,
                        ][(id % 5) as usize];
                        let held_tile = crate::net::HOTBAR
                            .get(player.held as usize)
                            .map(|(b, _)| crate::atlas::tiles_for_block(*b)[0]);
                        let swing = player.swing_phase.sin() * 0.7 * player.swing_amp;
                        (player.pos, player.yaw, shirt, held_tile, swing)
                    };
                    let verts = crate::mesh::player_model_mesh(shirt, held, pos, yaw, swing);
                    gpu.update_player(id, verts);
                }
                crate::net::PlayerEvent::Remove(id) => gpu.remove_player(id),
            }
        }
    }

    /// One frame: integrate movement, egui, then draw.
    fn frame(&mut self) {
        self.sync_world();
        self.sync_players();

        // egui frame: input → layout → tessellation.
        let ppp = self
            .gpu
            .as_ref()
            .map(|g| g.window.scale_factor() as f32)
            .unwrap_or(1.0);
        let (paint_jobs, textures_delta) = {
            // Take the egui state out so the paint closure can borrow all of
            // `self` without overlapping the state borrow (it goes back below).
            let Some(mut egui_state) = self.egui_state.take() else {
                return;
            };
            let ctx = egui_state.egui_ctx().clone();
            let window = Arc::clone(&self.gpu.as_ref().expect("gpu").window);
            let raw_input = egui_state.take_egui_input(&window);
            let full_output = ctx.run(raw_input, |ctx| self.paint_ui(ctx));
            egui_state.handle_platform_output(&window, full_output.platform_output);
            let paint_jobs = ctx.tessellate(full_output.shapes, ppp);
            self.egui_state = Some(egui_state);
            (paint_jobs, full_output.textures_delta)
        };

        // Player integration: gravity/walk (F toggles flight).
        let input = self.move_input();
        let dt = self
            .last_frame
            .take()
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_frame = Some(Instant::now());
        if let (Some(player), Some(camera)) = (self.player.as_mut(), self.camera.as_ref()) {
            let yaw = camera.yaw;
            let (fx, fz) = (yaw.cos(), yaw.sin());
            let mut wx = fx * input.forward - fz * input.strafe;
            let mut wz = fz * input.forward + fx * input.strafe;
            let len = (wx * wx + wz * wz).sqrt();
            if len > 1e-6 {
                wx /= len;
                wz /= len;
            }
            let spd = if player.flying {
                10.0
            } else {
                melon_world::WALK_SPEED
            };
            let wish = [wx * spd, wz * spd];

            if player.flying {
                player.feet[0] += wish[0] * dt;
                player.feet[1] += input.vertical * 10.0 * dt;
                player.feet[2] += wish[1] * dt;
                player.vel = [wish[0], input.vertical * 10.0, wish[1]];
            } else {
                let jumping = input.vertical > 0.0;
                let st = self.state.lock().expect("state");
                let r = melon_world::collision::step(
                    &st.world,
                    player.feet,
                    player.vel,
                    wish,
                    jumping,
                    dt.min(0.05),
                );
                drop(st);
                player.feet = r.feet;
                player.vel = r.velocity;
                player.on_ground = r.on_ground;
            }

            // Camera derives from the player: first-person eye, or a
            // third-person boom behind the head (F5).
            let eye = Vec3::new(
                player.feet[0],
                player.feet[1] + melon_world::PLAYER_EYE_HEIGHT,
                player.feet[2],
            );
            let camera = self.camera.as_mut().expect("camera checked above");
            camera.position = if self.third_person {
                eye - camera.forward() * 4.0
            } else {
                eye
            };

            // Walk-swing: phase advances with ground distance moved.
            let moved = self
                .prev_feet
                .map(|prev| {
                    let dx = player.feet[0] - prev[0];
                    let dz = player.feet[2] - prev[2];
                    (dx * dx + dz * dz).sqrt()
                })
                .unwrap_or(0.0);
            if player.on_ground && moved > 0.01 {
                self.swing_phase += moved * 2.4;
                self.swing_amp = (self.swing_amp + dt * 4.0).min(1.0);
            } else {
                self.swing_amp = (self.swing_amp - dt * 4.0).max(0.0);
            }
            self.prev_feet = Some(player.feet);
        }

        // Position reporting at a fixed rate (server relays to everyone).
        self.move_timer += dt;
        if self.move_timer >= MOVE_SEND_INTERVAL {
            self.move_timer = 0.0;
            let connected = self.state.lock().expect("state").connected;
            if let (Some(player), true) = (self.player.as_ref(), connected) {
                let camera = self.camera.as_ref().expect("camera");
                let _ = self
                    .edit_tx
                    .unbounded_send(melon_proto::ClientMessage::Move(melon_proto::PlayerMove {
                        x: player.feet[0],
                        y: player.feet[1],
                        z: player.feet[2],
                        yaw: camera.yaw,
                        pitch: camera.pitch,
                    }));
            }
        }

        // Local body in third person (invisible in first person).
        if let Some(gpu) = self.gpu.as_mut() {
            if self.third_person {
                if let (Some(player), Some(camera)) = (self.player.as_ref(), self.camera.as_ref()) {
                    let held_tile = crate::net::HOTBAR[self.selected_slot].0;
                    let held = crate::atlas::tiles_for_block(held_tile)[0];
                    let swing = self.swing_phase.sin() * 0.7 * self.swing_amp;
                    let verts = crate::mesh::player_model_mesh(
                        crate::atlas::tile::SHIRT,
                        Some(held),
                        player.feet,
                        camera.yaw,
                        swing,
                    );
                    gpu.update_player(u64::MAX, verts);
                }
            } else {
                gpu.remove_player(u64::MAX);
            }
        }

        let view_proj = match (&self.camera, self.gpu.as_ref()) {
            (Some(camera), Some(gpu)) => camera.view_proj(gpu.aspect()),
            _ => Mat4::IDENTITY,
        };
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.render(
                view_proj,
                crate::renderer::UiFrame {
                    paint_jobs,
                    textures_delta,
                    pixels_per_point: ppp,
                },
            );
        }
    }

    /// Broadcast the current hotbar selection.
    fn report_held(&mut self) {
        let _ = self
            .edit_tx
            .unbounded_send(melon_proto::ClientMessage::HeldItem {
                slot: self.selected_slot as u8,
            });
    }

    /// Break (left) / place (right) under the crosshair. Placement uses the
    /// selected hotbar block.
    fn block_edit(&mut self, place: bool) {
        let Some(camera) = self.camera else { return };
        let edit_tx = self.edit_tx.clone();
        let state = Arc::clone(&self.state);
        let place_block = melon_world::block::AIR;
        let msg = {
            let st = state.lock().expect("state");
            let place_block = if place {
                crate::net::HOTBAR[self.selected_slot].0
            } else {
                place_block
            };
            st.pick_edit(
                camera.position.to_array(),
                camera.forward().to_array(),
                place,
                place_block,
            )
        };
        if let Some(msg) = msg {
            let _ = edit_tx.unbounded_send(msg);
        }
    }

    /// egui painting: status header + chat log + input field.
    fn paint_ui(&mut self, ctx: &egui::Context) {
        let (server_name, player_count, ready, chat_tail) = {
            let st = self.state.lock().expect("state");
            (
                st.server_name.clone(),
                st.players.len() + usize::from(st.ready),
                st.ready,
                st.chat_lines
                    .iter()
                    .rev()
                    .take(CHAT_TAIL)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        };

        egui::TopBottomPanel::top("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(0x9d, 0xff, 0x57), "🍈 MELON");
                ui.separator();
                ui.label(format!("{server_name} · {player_count} players"));
                if !ready {
                    ui.colored_label(egui::Color32::YELLOW, "streaming world…");
                }
            });
        });

        // Hotbar: centered above the chat panel.
        egui::Area::new(egui::Id::new("hotbar"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -150.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (i, (block, label)) in crate::net::HOTBAR.iter().enumerate() {
                        let selected = i == self.selected_slot;
                        let color = block_color(*block);
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(44.0, 44.0), egui::Sense::click());
                        if selected {
                            ui.painter().rect_filled(
                                rect.expand2(egui::vec2(3.0, 3.0)),
                                2.0,
                                egui::Color32::from_rgb(0x9d, 0xff, 0x57),
                            );
                        }
                        ui.painter().rect_filled(rect.shrink(3.0), 2.0, color);
                        ui.painter().text(
                            rect.right_bottom(),
                            egui::Align2::RIGHT_BOTTOM,
                            format!("{}", i + 1),
                            egui::FontId::proportional(11.0),
                            egui::Color32::WHITE,
                        );
                        if response.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }
                        let response = response.on_hover_text(*label);
                        if response.clicked() {
                            self.selected_slot = i;
                        }
                    }
                });
            });

        egui::TopBottomPanel::bottom("chat").show(ctx, |ui| {
            ui.set_min_height(140.0);
            ui.add_space(4.0);
            // Chat log: newest at the bottom.
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for line in &chat_tail {
                        ui.label(line);
                    }
                });
            ui.add_space(4.0);
            // Input row: hint vs live text field.
            if self.chat_focused || self.chat_focus_requested {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.chat_input)
                        .desired_width(f32::INFINITY)
                        .hint_text("chat — Enter to send, Esc to cancel"),
                );
                if self.chat_focus_requested {
                    response.request_focus();
                    self.chat_focus_requested = false;
                }
                let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                if (enter && response.has_focus()) || response.lost_focus() {
                    let text = self.chat_input.trim().to_string();
                    self.chat_input.clear();
                    self.chat_focused = false;
                    if !text.is_empty() {
                        let _ = self
                            .edit_tx
                            .unbounded_send(melon_proto::ClientMessage::Chat { text });
                    }
                }
            } else {
                ui.label(egui::RichText::new("press T to chat · LMB break · RMB place").weak());
            }
        });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, elwt: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        let window = Arc::new(
            elwt.create_window(
                Window::default_attributes()
                    .with_title("MELON — connected world (Phase 4)")
                    .with_inner_size(LogicalSize::new(1280.0, 720.0)),
            )
            .expect("create window"),
        );
        let gpu = pollster::block_on(Gpu::new(window)).expect("init wgpu");

        let egui_state = egui_winit::State::new(
            egui::Context::default(),
            egui::ViewportId::ROOT,
            &gpu.window,
            Some(gpu.window.scale_factor() as f32),
            None,
            None,
        );
        self.egui_state = Some(egui_state);

        self.grab_cursor(&gpu.window);
        self.last_frame = Some(Instant::now());
        self.gpu = Some(gpu);
        println!("window up — world streaming; T chat · LMB break · RMB place · ESC quit");
    }

    fn window_event(&mut self, elwt: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        // Feed every event to egui first — it consumes text input, panel
        // clicks, and reports whether a repaint is needed.
        if let Some(egui_state) = self.egui_state.as_mut() {
            let window = self
                .gpu
                .as_ref()
                .map(|g| Arc::clone(&g.window))
                .expect("gpu exists once egui does");
            let _response = egui_state.on_window_event(&window, &event);
        }

        // Window-level events that need no GPU borrow.
        match &event {
            WindowEvent::CloseRequested => {
                elwt.exit();
                return;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if event.logical_key == Key::Named(NamedKey::Escape) {
                        if self.chat_focused {
                            // Esc closes the chat instead of quitting.
                            self.chat_focused = false;
                            self.chat_input.clear();
                            return;
                        }
                        elwt.exit();
                        return;
                    }
                    let is_t = matches!(event.logical_key, Key::Character(ref c) if c.eq_ignore_ascii_case("t"));
                    if is_t {
                        if !self.chat_focused {
                            self.chat_focused = true;
                            self.chat_focus_requested = true;
                        }
                        return;
                    }
                    // Hotbar selection: number keys 1..9.
                    if let PhysicalKey::Code(code) = event.physical_key {
                        let slot = match code {
                            KeyCode::Digit1 => Some(0),
                            KeyCode::Digit2 => Some(1),
                            KeyCode::Digit3 => Some(2),
                            KeyCode::Digit4 => Some(3),
                            KeyCode::Digit5 => Some(4),
                            KeyCode::Digit6 => Some(5),
                            KeyCode::Digit7 => Some(6),
                            KeyCode::Digit8 => Some(7),
                            KeyCode::Digit9 => Some(8),
                            _ => None,
                        };
                        if let Some(slot) = slot {
                            self.selected_slot = slot;
                            self.report_held();
                            return;
                        }
                    }
                    // F toggles flight, F5 toggles the third-person camera.
                    if event.physical_key == PhysicalKey::Code(KeyCode::KeyF) {
                        if let Some(player) = self.player.as_mut() {
                            player.flying = !player.flying;
                            player.vel[1] = 0.0;
                        }
                        return;
                    }
                    if event.physical_key == PhysicalKey::Code(KeyCode::F5) {
                        self.third_person = !self.third_person;
                        return;
                    }
                    if !self.chat_focused {
                        self.keys.insert(event.physical_key);
                    }
                } else {
                    self.keys.remove(&event.physical_key);
                }
                return;
            }
            WindowEvent::Focused(focused) => {
                let Some(window) = self.gpu.as_ref().map(|g| Arc::clone(&g.window)) else {
                    return;
                };
                self.keys.clear();
                if *focused {
                    self.grab_cursor(&window);
                } else {
                    self.release_cursor(&window);
                }
                return;
            }
            _ => {}
        }

        let Some(gpu) = self.gpu.as_mut() else { return };
        if window_id != gpu.window.id() {
            return;
        }

        match event {
            WindowEvent::Resized(size) => gpu.resize(size),
            WindowEvent::CursorMoved { position, .. } => {
                if self.chat_focused {
                    return; // cursor belongs to the chat UI
                }
                if !self.mouse_captured || self.skip_warp_event {
                    self.skip_warp_event = false;
                    return;
                }
                let size = gpu.window.inner_size();
                let center =
                    PhysicalPosition::new(size.width as f64 / 2.0, size.height as f64 / 2.0);
                let dx = position.x - center.x;
                let dy = position.y - center.y;
                if dx.abs() < 0.5 && dy.abs() < 0.5 {
                    return; // already centered — nothing to feed the camera
                }
                let Some(camera) = self.camera.as_mut() else {
                    return;
                };
                camera.look(dx as f32, dy as f32);
                // Warp back to the middle; skip the event that warp fires.
                self.skip_warp_event = true;
                let _ = gpu.window.set_cursor_position(center);
            }
            WindowEvent::MouseInput {
                state: mouse_state,
                button,
                ..
            } => {
                if mouse_state == ElementState::Pressed && !self.chat_focused {
                    self.block_edit(matches!(button, MouseButton::Right));
                }
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, elwt: &ActiveEventLoop) {
        // Network death → report and leave gracefully.
        let failure = {
            let st = self.state.lock().expect("state");
            if st.error.is_some() && !st.connected {
                st.error.clone()
            } else {
                None
            }
        };
        if !self.exited {
            if let Some(err) = failure {
                eprintln!("session ended: {err}");
                self.exited = true;
                elwt.exit();
                return;
            }
        }
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
