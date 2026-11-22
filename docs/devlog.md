# MELON Dev Log — Phase Notes

> The engineering journal: **what actually happened in each phase** — decisions, gotchas, and lessons — the context between the commits.
> Strategy & rationale → [engine-plan.md](engine-plan.md) · checkbox state → [plan.md](../plan.md) · reference policy → [reference/minecraft-server-1.8.9.md](reference/minecraft-server-1.8.9.md).

| Phase | Status | Date | Tests |
|---|---|---|---|
| 0 — Workspace & CI | ✅ complete | 2026-11-22 | — |
| 1 — Protocol spine | ✅ complete | 2026-11-22 | 26 green |
| 2 — Client renders a world | ✅ complete | 2026-11-22 | 46 green |

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

## Phase 3 preview — Connected World

Next: the server owns the world. `ChunkData` messages stream over `melon-proto`, the client caches + re-meshes dirty sections, and block place/break becomes a validated server-side broadcast. The mesher's world-space vertices and the protocol's versioned handshake are already shaped for this — the server just needs a session loop that sends sections after auth.

---

## Demo / smoke-test commands

```bash
cargo test --workspace                                  # all gates (46 tests)
cargo run -p melon-server                               # starts on 127.0.0.1:25777
cargo run -p melon-client -- --headless --name YourName # handshake + ping RTT
cargo run -p melon-client                               # Phase 2 world window (ESC to quit)
cargo run -p melon-client -- --screenshot /tmp/frame.png # offscreen visual gate
```
