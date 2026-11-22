# MELON — Execution Plan

> **Working tracker.** Strategy, architecture rationale, and risk analysis live in [docs/engine-plan.md](docs/engine-plan.md). Per-phase engineering notes (decisions, gotchas, lessons) live in [docs/devlog.md](docs/devlog.md).
> Product: a lightweight Minecraft-style **client + server platform in Rust**, speaking MELON's own custom protocol. BedWars first. Vanilla 1.8.9 arrives later as a bridge (M10).
>
> Every milestone ends with a hard acceptance test. A milestone is "done" when the test passes — not when the code exists.

---

## Phase 0 — Workspace & CI

**Goal: a compiling 5-crate Cargo workspace with quality gates wired.**

- [x] Rust toolchain installed (stable + clippy + rustfmt)
- [x] Cargo workspace: `crates/melon-proto`, `melon-world`, `melon-client`, `melon-server`, `melon-tools`
- [x] Shared dependency table in workspace `Cargo.toml`
- [x] Root `.gitignore` (`/target`, `.DS_Store`, editor noise)
- [x] GitHub Actions CI: `cargo fmt --check` + `cargo clippy -D warnings` + `cargo test`
- [x] `README.md` project structure updated to the real layout

## Phase 1 — Protocol spine ✅

**Goal: two CLI binaries speaking the real versioned protocol over TCP.**
**Acceptance: server and client complete handshake → auth → ping/pong round-trip; version mismatch is rejected with a reason; `cargo test` proves it.**

- [x] `melon-proto`: VarInt codec (LEB128 u32) with round-trip tests
- [x] `melon-proto`: length-prefixed frame codec (`FrameCodec<M>`) over `tokio_util::codec`, generic over message type
- [x] `melon-proto`: `ClientMessage` / `ServerMessage` v1 enums + `PROTOCOL_VERSION = 1`
- [x] `melon-proto`: message round-trip tests (handshake, auth result, ping/pong)
- [x] `melon-server`: TCP listener, per-connection task, handshake validation (reject mismatched version + reason), ping→pong
- [x] `melon-client --headless`: connect, handshake, RTT ping loop (`--host/--port/--name/--count`)
- [x] `melon-world`: block registry + chunk sections + flat-world generator (unit tested) — foundation for Phase 3
- [x] Integration test: in-process server + client, handshake + ping assertions
- [x] Demo run: server + headless client, real RTT numbers printed

## Phase 2 — Client renders a world ✅

**Goal: `melon-client` (GUI mode) opens a window and renders the flat world.**
**Acceptance: 60+ FPS fly-cam over a flat world on a laptop iGPU; meshing benchmark lives in `cargo test`.**

- [x] `winit` window + `wgpu` surface/device setup, clear-color frame, ESC to quit
- [x] Procedural texture atlas (runtime-generated, own art only — deterministic, half-texel inset, missing-texture checker)
- [x] Chunk section mesh pipeline — culled meshing v1 (neighbor face culling; greedy merge is a follow-up optimization, benchmark keeps cost honest)
- [x] Depth buffer + textured terrain render of `melon-world`'s flat world (world-space verts, baked face shading)
- [x] Fly camera (WASD/Space/Shift + mouse-look with focus-safe cursor grab and pitch clamping)
- [x] Mesh benchmark test (16 sections within budget in debug builds)
- [x] Offscreen `--screenshot` visual gate — renders the spawn view to PNG, verifies winding/UV/shading without macOS permissions (CI-able)

*Interactive fly-cam feel gets human eyes: `cargo run -p melon-client`.*

## Phase 3 — Connected World ✅

**Goal: the server owns the world; clients see and edit it.**
**Acceptance: two clients edit the same server-side world and agree (e2e proves the broadcast path; two windows work live).**

- [x] `melon-world`: chunk wire codec (`encode_sections`/`decode_sections` — non-empty sections only, 8 KB per superflat chunk) + voxel raycast (Amanatides–Woo DDA, unit tested)
- [x] Protocol v1 additions: `WorldInfo` · `ChunkData` · `SetBlock` · `BlockUpdate`
- [x] `melon-server`: `WorldHandle` (shared world + sender registry) — post-auth world stream, validated edits, broadcast to every client, `--world-size` flag
- [x] `melon-client`: background network thread + shared `ClientState`; chunks → world cache → dirty sections → GPU re-mesh
- [x] Block place/break: raycast pick (LMB break / RMB place) → server validation → broadcast → all clients re-mesh
- [x] e2e: world stream shape, cross-client broadcast, invalid-edit silent rejection + session survival
- [x] Acceptance demo: headless client streams 16 chunks (131 KB), pings, and verifies its edit echoes back

## Phase 4 — Presence & chat

- [ ] PlayerJoin/Leave/Move broadcast; remote players rendered
- [ ] Chat messages (text only) with rate limiting
- [ ] Acceptance: two windows see each other move and chat

## Phase 5 — Inventory & polish ✅

- [x] Hotbar UI (egui): 9-block palette, selection via number keys / scroll wheel / click
- [x] Held-item sync: `HeldItem` → server relay → `PlayerHeld` — the held block renders as a cube at the player's hand
- [x] Block placement uses the selected hotbar slot
- [x] Walk/limb swing animation (phase advances with distance moved, settles when standing)
- [x] Acceptance: hotbar select shows the right held item remotely (e2e: held relay stamped with sender id, invalid slots dropped)

## Phase 6 — Combat

- [ ] Swing/hit/velocity messages, server-side hit validation (ray trace)
- [ ] 1.8 knockback constants (from reference analysis) as tunable config
- [ ] Acceptance: duel feels correct; clamps reject impossible movement

## Phase 7 — Arena format & real maps

- [ ] `melon-arena` container: blocks + spawn points + team data
- [ ] Importer from vanilla-built maps (`.schematic` / `.mca` read-only)
- [ ] Acceptance: a real BedWars map built in vanilla loads and renders

## Phase 8 — BedWars vertical slice

- [ ] Teams, beds, generators, shop UI, win condition, match FSM
- [ ] Acceptance: a full match completes between two clients

## Phase 9 — Polish

- [ ] Audio (`kira`), particles, art pass (commission/CC0), settings, reconnect
- [ ] Acceptance: feels like a game, not a demo

## Phase 10 — Vanilla bridge

- [ ] Gateway translating Protocol 47 ⇔ melon-proto (uses wiki.vg + reference jar analysis)
- [ ] Acceptance: a vanilla 1.8.9 client joins a MELON match

## Phase 11 — Density & benchmarks

- [ ] Multi-match runtime, public benchmark suite, headless bot load test
- [ ] Acceptance: publish players/core + matches/machine numbers

---

## Standing rules

1. **Acceptance tests are king.** No milestone is done on vibes.
2. **One actor per match** — the sim never shares locks.
3. **Zero unsafe by default**; any exception quarantined to codec internals with justification.
4. **Clean-room always**: no Mojang code, assets, or data files — ever.
5. **Perf gates early**: the Phase 2 mesh benchmark prevents silent perf debt.
6. Version every protocol message set from the first byte (Phase 1 does this).
