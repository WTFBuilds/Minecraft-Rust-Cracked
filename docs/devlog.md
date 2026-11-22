# MELON Dev Log — Phase Notes

> The engineering journal: **what actually happened in each phase** — decisions, gotchas, and lessons — the context between the commits.
> Strategy & rationale → [engine-plan.md](engine-plan.md) · checkbox state → [plan.md](../plan.md) · reference policy → [reference/minecraft-server-1.8.9.md](reference/minecraft-server-1.8.9.md).

| Phase | Status | Date | Tests |
|---|---|---|---|
| 0 — Workspace & CI | ✅ complete | 2026-11-22 | — |
| 1 — Protocol spine | ✅ complete | 2026-11-22 | 26 green |
| 2 — Client renders a world | ✅ complete | 2026-11-22 | 46 green |
| 3 — Connected World | ✅ complete | 2026-11-22 | 59 green |
| 4 — Presence, chat & the player body | ✅ complete | 2026-11-23 | 67 green |

---

## Phase 0 — Workspace & CI

**Shipped:** 5-crate Cargo workspace, shared dependency table, root `.gitignore`, GitHub Actions (Rust gates + website build), README structure refresh.

**Important notes & decisions:**

- **Rust toolchain wasn't on the machine** — installed via `rustup` (stable, with clippy + rustfmt). Anyone contributing needs the same: `curl https://sh.rustup.rs | sh`.
- **Layout decision:** member crates live in `crates/melon-*`. The `melon-` prefix matters — `melon-proto` imported side-by-side with third-party crates reads unambiguously in `use` blocks.
- **Edition 2021** (not 2024) on purpose: zero benefit for this code today, maximum compatibility for contributors and CI images.
- **Workspace deps live in one table** (`[workspace.dependencies]`): version drift between crates is impossible, and adding a crate is one line.
- **CI is dual-job**: Rust gates (`fmt --check`, `clippy -D warnings`, `test`) *and* the Next.js website (lint + build) — one repo, one green check.
- **Clippy noise note:** `block v0.1.6` (transitive macOS dep of winit) emits a *future-incompat* warning — not our code, expected, ignore it.

---

## Phase 1 — Protocol spine

**Shipped:** `melon-proto` (VarInt, framing, v1 messages), `melon-server` (listener + handshake/auth/ping), `melon-client --headless`, `melon-world` (blocks/chunks/superflat), `melon-tools` stub, 5 e2e tests over real TCP.

### Key decisions

1. **The big one — perspective codecs.** My first cut was a generic `FrameCodec<M>` with `impl<M: Serialize> Encoder<M> for FrameCodec<M>`. That is a trap: the server must **read `ClientMessage`s but write `ServerMessage`s over the same connection**, and `Framed<TcpStream, FrameCodec<ClientMessage>>` will only ever give it `Sink<ClientMessage>`. One `Framed` = one codec = one Item type. The fix that makes the whole platform cleaner: **perspective codecs** —
   - `ServerSessionCodec`: decodes `ClientMessage`, encodes `ServerMessage`
   - `ClientSessionCodec`: the exact mirror

   Direction confusion is now *unrepresentable*: you cannot accidentally send a client message from the server, the compiler forbids it. This shape carries straight through Phase 3+ (same pattern, more message types).
2. **Version negotiation on the first message, with a reason.** The handshake carries `protocol_version`; a mismatch gets an explicit `ProtocolVersionMismatch { expected, got }` reply *then* a clean close. Never guess, never silently bridge.
3. **Frame cap = 1 MiB.** A length prefix is an attack surface; oversized frames are rejected before allocation. Chunk payloads in Phase 3 must stay under it by design.
4. **Non-consuming VarInt peek.** The decoder reads the length prefix *without consuming* and only `split_to`s when the full payload has arrived — partial TCP deliveries are buffered correctly by construction (covered by a chunked-delivery test that feeds the frame 3 bytes at a time).
5. **Username rules live in `melon-proto`,** not the server: both sides (and future tools) share one validator — trim, 1–16 chars, `[A-Za-z0-9_-]` only.
6. **Duplicate handshake on a live session is a protocol violation** → connection dropped, never treated as a re-auth.

### Gotchas hit (and fixed)

- **17 errors from one missing line.** error.rs lacked `use thiserror::Error;` — the derive and every `#[from]` vanished, cascading into "trait bound not satisfied" errors everywhere. Lesson: with derive-heavy code, *read the first error only*; the rest is usually fallout.
- **`write_varint(&mut Vec<u8>)` vs `BytesMut`.** The frame encoder writes into a `BytesMut`; rather than contort the VarInt API, the encoder inlines a 4-line LEB128 loop via `bytes::BufMut::put_u8`.
- **Debug-build arithmetic traps are a gift.** The e2e test used `1000 + u64::MAX` as a timestamp — panics with "attempt to add with overflow" in debug. Fixed with `wrapping_add`. Debug overflow checks just caught a real protocol-edge bug for free.
- **Off-by-one in my own layer math.** Superflat = bedrock y0 · stone y1–2 · dirt y3–4 · grass y5 · air y6+. My test asserted grass at y6 — the *test* was wrong, not the generator. Lesson: when a generator and its test disagree, recompute the spec before touching either.

### Numbers

- 26 tests green · `cargo fmt --check` clean · `cargo clippy -D warnings` clean.
- Localhost demo (release-independent, debug build): handshake accepted, MOTD returned, **RTT 45–53 µs** across 4 pings, session lifecycle logged via `tracing`.
- One debug-mode caveat: `wgpu` pulls in `block v0.1.6` which is future-incompat flagged — informational only.

---

## Phase 2 — Client renders a world ✅

**Shipped:** procedural texture atlas, culled section mesher, fly camera, terrain pipeline (WGSL), full GUI integration — and the **first rendered frame**, verified visually via the new offscreen screenshot gate.

### Key decisions

1. **Culled meshing v1, greedy merge later.** Faces exist only where solid meets air. Full greedy merging is a *known, deferred* optimization: the vertex format and draw path are already greedy-ready, and the benchmark gate keeps meshing cost honest in the meantime. This ordered "correct before clever."
2. **Face winding proven by math, not vibes.** Each face's corners were wound so `(p1−p0) × (p2−p0) · normal > 0` — derived by hand for all six faces before writing the table. Result: back-face culling worked on the very first render (no flipped-face debugging session).
3. **Baked directional shade instead of lights.** Top 1.0 · ±Z 0.85 · ±X 0.72 · bottom 0.55 — instant voxel depth for free, carried in a 4-byte vertex attribute.
4. **Non-indexed quads:** `QUAD_INDEX_PATTERN = [0,1,2,0,2,3]` expands 4 corners into 6 vertices per face. Indexed buffers are a trivial later swap if vertex count ever matters.
5. **Atlas = deterministic hash noise.** Same bytes every run/machine (tested). Half-texel UV inset kills tile bleeding with nearest sampling. **Unused slots render as a magenta/black missing-texture checker** — a bug you can *see* instead of geometry that silently vanishes.
6. **The offscreen screenshot gate.** macOS `screencapture -l` automation hit permission walls (System Events needs accessibility, window capture needs screen-recording). Instead: `melon-client --screenshot path.png` renders one frame offscreen (no surface, no window, no permissions) and writes a PNG through a **minimal hand-rolled PNG encoder** (stored zlib + adler32 — zero new dependencies). This is now the CI-able visual gate for every rendering change.
7. **`glam::Mat4::perspective_rh` (0..1 depth)** is the wgpu-correct projection — `_gl` variants map to OpenGL's −1..1 and would silently break depth.
8. **Sequential borrows in the frame path.** The event loop computes `view_proj` → `move_input()` → `apply_move()` → `render()` as separate steps instead of holding one `&mut self.gpu` across them — zero borrow fights, zero `RefCell`s.

### Gotchas hit (and fixed)

- **The mesher's first bug was caught by tests, not eyes:** it emitted 4 corners per face while draws needed 6 vertices (two triangles). `single_block_has_six_faces` failed with 20 vs 36 before anything ever rendered. Then the *test expectations* were wrong twice (bottom faces are culled at y=0; superflat grass lives at y=5) — recomputed the spec before touching code again.
- **The atlas opacity test caught transparent unused tiles** → missing-texture checker added.
- **winit 0.30.13:** `LogicalKey` is `Key` now; `ActiveEventLoop::exit()` takes no arguments; keys come via `PhysicalKey::Code(KeyCode::…)` (no `winit::keyboard::Code`).
- **wgpu 25:** `Instance::new(&InstanceDescriptor)` by reference; `SurfaceConfiguration` *requires* `desired_maximum_frame_latency`; `BindingType::Sampler` is a tuple variant.
- **macOS screenshot permissions** are un-automatable for `screencapture -l` — that dead end produced the screenshot gate instead.
- **Background processes die with the bash call** that spawned them — window smoke tests must launch, verify, and kill within a single command.

### Numbers

- 46 tests green (atlas 6 · mesher 9 incl. budget gate · camera 6 · proto 14 · e2e 5 · world 7) · fmt + clippy `-D warnings` clean.
- Superflat 4×4 chunks meshes to **16 sections / 22,528 vertices**.
- First frame verified: grass texture ✓, perspective ✓, depth ✓, face culling ✓, shading ✓, night sky ✓.

### API notes baked into the code (winit 0.30.13 / wgpu 25)

- `winit::keyboard::LogicalKey` → **`Key`**; ESC = `Key::Named(NamedKey::Escape)`; `ActiveEventLoop::exit()` takes **no arguments**.
- `wgpu::Instance::new(&InstanceDescriptor)` (by reference); `SurfaceConfiguration` **requires** `desired_maximum_frame_latency`; `BindingType::Sampler(SamplerBindingType)` is a tuple variant; `request_device(&DeviceDescriptor)` is single-argument.
- `ApplicationHandler::window_event` gives `&self` — take `self.gpu.as_mut()` first and match on the event after; **`Surface<'static>`** requires `Arc<Window>`; on `get_current_texture` failure, reconfigure and wait for the next redraw.

---

## Phase 3 — Connected World ✅

**Shipped:** the server owns the world. After auth it streams every chunk; clients re-mesh dirty sections locally; left/right clicks raycast-pick a block, the server validates the edit and broadcasts it to everyone.

### Key decisions

1. **Chunk wire format: non-empty sections only.** A raw chunk is 128 KB; a superflat chunk has exactly one populated section → 8 KB + a 16-bit section mask. The full 4×4 dev world travels as 131 KB. Encoding/decoding lives in `melon-world` (`encode_sections`/`decode_sections`) so client and server can never drift — and all-air sections recreate as empty on decode.
2. **Perspective codecs paid off immediately.** The Phase 3 server session splits the framed connection: a per-connection **writer task** drains an outbound channel, and a **sender registry** (`WorldHandle`) fans broadcasts out to every client. Direction confusion stayed unrepresentable, and adding the broadcast path was registry + one writer task.
3. **Shared dev world, not an actor (yet).** `WorldHandle` is a `World` behind a mutex + registry — deliberate: this is the lobby-style world, and the *one actor per match* rule applies to the match runtime in Phase 8. Documented, not forgotten.
4. **No client-side prediction.** Clicks send `SetBlock`; the server validates (world bounds), applies, and broadcasts a `BlockUpdate` that **every** client — including the editor — applies. Idempotent, correct, and on LAN the latency is invisible. Prediction/rollback arrives when latency makes it matter.
5. **Dirty-section marking includes boundary neighbors.** A block at local x=0 changes faces in the neighboring section too — the client marks section ±1 along every axis whose local coordinate sits on a border. Missing this is the classic "hole at chunk boundary" bug.
6. **Voxel raycast (Amanatides & Woo DDA) lives in `melon-world`,** dependency-free (`[f32;3]` in/out) — the client uses it for picking, Phase 6 server hit-validation will reuse it. Starting-voxel-solid returns `None` (nothing sensible to pick).
7. **Headless client grew the full Phase 3 flow** — it streams the world, pings, places a block, and verifies its own edit echoes. `./setup.sh demo` is now a complete protocol exercise, and it doubles as the base for load-test bots.

### Gotchas hit (and fixed)

- **The DDA raycast had a real bug:** the loop never advanced `t_max` by `t_delta` — it would walk the same boundary forever. Caught because clippy flagged `t_delta` as unused *before any test ran*. Then the sideways-hit test caught a second subtlety: in a superflat world a horizontal ray at grass height **starts inside a solid block** — the test was rewritten against a single-block world.
- **World stream changed every existing e2e test:** the server now sends `WorldInfo` + chunks *before* session replies, so tests that expected an immediate `Pong` broke. The `read_world` helper consumes the stream and skips the handshake result — lesson: when the protocol gains a mandatory sequence, update the test harness's flow helpers, not each assertion.
- **`Chunk` isn't `PartialEq`** (16 KB of blocks) — error-path tests use `matches!` instead of `assert_eq!` on `Result<Chunk>`.
- **`WorldInfo` carries an `f32`** → the `Eq` derive on `ServerMessage` had to go (partial_eq is enough).
- **fmt churn between edits:** several Write-attempt failures because `cargo fmt` reformat files between operations — always Read-before-Write after running fmt.
- **Unclosed delimiter from a surgical edit** — one missing `mod tests` brace surfaced as a single cryptic error; fixed by reading the tail of the file.

### Live-fire bugs (found the moment a human ran the client)

The e2e suite was green, but the first real `./setup.sh client` produced **"nothing happens"** — process alive, no window. Two bugs were hiding behind the tests:

1. **Same-statement mutex deadlock.** `println!("…", state.lock().….server_name, state.lock().….motd)` — two `lock()` temporaries in *one statement* live until the statement ends, so the second lock deadlocked against the first **on the same thread**. It fired *before* window creation → no window, no error, just a hung process. Rule learned: **one lock scope per statement, always `{ let st = …; }`.** Found via `sample <pid>` — the main thread was parked in `pthread_mutex_lock` at gui.rs:40.
2. **A tokio `TcpStream` is welded to the runtime that created it.** `connect_and_auth` ran on the GUI thread's *temporary* runtime; the connection then moved to the net thread's runtime — whose reads failed with *"A Tokio 1.x context was found, but it is being shutdown"* as soon as the first runtime dropped. Fix: connect + auth + session all live on the **one** network thread/runtime; the GUI learns the auth outcome through a `std::sync::mpsc` channel and fails fast before opening a window. (Bonus: connect now has a 5 s timeout with a "is the server running?" hint.)

Verification after both fixes: window visible via `CGWindowList` (`screencapture -R` shows the **server-streamed** world rendering — grass/dirt/stone/bedrock layers at the world edge), plus the log sequence `connected → window up`.

**Standing rules added:** never hold a `MutexGuard` across another lock acquisition in the same statement; never move a tokio IO object across runtimes — spawn the IO *on* the runtime that will own it, and communicate results through channels.

### Numbers

- **59 tests green** (world 16 · proto 15 · client 20 · server e2e 8) · fmt + clippy `-D warnings` clean.
- World stream: **16 chunks · 131,072 bytes** (exactly 16 × 8192 — the math checks out on the wire).
- Demo: auth → world → 4 pings (~150–420 µs) → `SetBlock (2,6,2) wool:red` → broadcast echo verified.
- GUI: connects, streams, renders, edits — verified alive with the server running.

---

## Phase 4 — Presence, chat & the player body ✅

**Shipped:** presence (join/roster/leave with server-assigned ids), chat (egui panel, validated + rate-limited), the Minecraft-style player body (head/torso/arms/legs, yaw-oriented), and **real player physics** — gravity, AABB-vs-voxel collision, jumping. Players now stand on blocks.

### Key decisions

1. **Server-assigned player ids in the handshake reply.** The client learns `player_id` in `HandshakeResult` and filters its own broadcast echoes (moves/chat) client-side — the server broadcasts to everyone including the sender, which keeps the server dumb and the client logic local.
2. **Per-player spawn offsets.** `spawn_for(id)` shifts each player 3 blocks along x so simultaneous joins don't stack inside each other.
3. **Throttling lives on the server:** movement relays at most one `PlayerMove` per 50 ms per player; chat max 5 per 10 s + length/control-character validation. Clients stay honest; the server stays calm.
4. **egui entered the stack** (egui + egui-wgpu + egui-winit 0.32 — the version that pins wgpu 25, our exact version). Status header + chat panel; the chat field captures the keyboard while focused (T to open, Enter to send, Esc to cancel) and camera input is suppressed meanwhile.
5. **Physics lives in `melon-world`** (`collision::step`): per-axis resolution (Y → X → Z) so wall-sliding works, gravity 32 blocks/s², jump velocity 9 (clears 1.25 blocks), walk speed 4.317. Feet-based position; the camera derives eye = feet + 1.62.
6. **The body model is 6 boxes with Minecraft proportions** (legs 0.75, torso 0.75, head 0.5), assembled in local space, yaw-rotated at build time (~216 verts per player, rebuilt on every move — trivial). Skin set in the atlas: skin/face/hair/shirt/pants tiles; shirt color varies per player id.

### Gotchas hit (and fixed)

- **THE big one — the part-layout axis bug.** My first body placed the arm/leg offsets along **X — the facing axis** — instead of Z (left-right). From the front view the arms ended up in front of / behind the torso: the right arm (closer) occluded it, the left arm + one leg hid behind it, and the whole body rendered as a thin "stick". The mesh probes kept saying the data was correct — because it was — the LAYOUT was wrong, not the code. Diagnosed by pixel-scanning the rendered columns (one 0.27-block skin run + one 0.22-block pants run = exactly one arm + one leg, centered) and by a vertex histogram grouped by (height band, atlas tile). **Lesson: a "wrong render" with a "correct mesh" means the layout is wrong — check which axis your offsets are on.**
- **Same-statement mutex deadlock** (`println!("…", state.lock()…, state.lock()…)`) — two lock temporaries in one statement deadlock on the same thread. Fired before window creation → "nothing happens". Found via `sample <pid>`.
- **A tokio `TcpStream` is welded to the runtime that created it.** Connect+auth ran on the GUI thread's temporary runtime; the session then moved the connection to the net thread's runtime → *"A Tokio 1.x context was found, but it is being shutdown"*. Fix: connect+auth+session all on the ONE network thread; the GUI learns the auth outcome through a channel.
- **The atlas checker pass ate the skin tiles** — the "known tiles" guard only protected row 0, so the checker overwrote row 1 (skin set) with the missing-texture checker. The atlas test caught it (opacity per pixel).
- **Two of my own test bugs:** `1000 + u64::MAX` overflow (wrapping_add), and a histogram with `[f32::INFINITY; 2]` used for BOTH min and max tracking (max slot never updated). Also an off-by-one in the superflat layer expectations.
- **egui/winit/wgpu version dance:** egui-wgpu 0.32.3 pins wgpu 25.0.2 — exactly ours (checked with `cargo tree`). winit 0.30: `Key::Character(SmolStr)` compares via `matches!` + `eq_ignore_ascii_case`; `ActiveEventLoop::exit()` takes no args; `SurfaceConfiguration` requires `desired_maximum_frame_latency`.
- **egui borrow pattern:** take the `egui_winit::State` out of its `Option` for the frame (`take()` → run → put back) so the paint closure can borrow all of `App` without overlapping the state borrow.

### Numbers

- **67 tests green** (world 21 incl. 5 collision · proto 16 · client 22 · e2e 8) · fmt + clippy `-D warnings` clean.
- Physics verified by simulation tests: landing exactly on y=6, jump apex 1.25 blocks, wall clamp, flat-ground stability, void-fall documentation.
- Model verified visually via the offscreen screenshot gate: face + eyes, hair, shirt, arms, pants — correct proportions and yaw.

---

## Phase 5 preview — Inventory & polish

Next: egui hotbar + held-item sync (the UI layer is live), limb swing animation for the model, then Phase 6 combat (the collision module is what hit-validation will reuse).

---

## Demo / smoke-test commands

```bash
./setup.sh                       # build + demo: server starts, headless client streams world + edits
./setup.sh server                # server in the foreground (--world-size via MELON_WORLD_SIZE)
./setup.sh client                # connected world window (LMB break · RMB place · ESC quit)
./setup.sh play                  # server (background) + window together, auto-cleanup
./setup.sh headless              # world stream + pings + block-edit echo on stdout
./setup.sh screenshot            # offscreen visual gate → /tmp/melon-frame.png
./setup.sh test                  # fmt + clippy -D warnings + all tests

# raw cargo equivalents
cargo test --workspace                                   # all gates (59 tests)
cargo run -p melon-server                                # starts on 127.0.0.1:25777
cargo run -p melon-client -- --headless --name YourName  # world + pings + block edit
cargo run -p melon-client -- --screenshot /tmp/frame.png # offscreen visual gate
```
