# MELON Platform Plan — Lightweight Client + Server in Rust

> **Product:** a self-contained Minecraft-style arcade platform — MELON's own lightweight **playable client** and lightweight **server**, speaking **MELON's own clean binary protocol**. Not a vanilla replica: only the gameplay BedWars needs. Vanilla 1.8.9 support arrives later as a translation bridge.
>
> Decisions locked 2026-11-22: **playable client first · custom protocol now · vanilla bridge later.**
>
> Background reference: [vanilla 1.8.9 jar analysis](reference/minecraft-server-1.8.9.md) (kept for feel constants, translation keys, and the future bridge phase).

---

## 1. Why this direction is lighter than a vanilla replica

| A vanilla 1.8.9 replica must have | MELON custom platform must have |
|---|---|
| Full Protocol 47 surface (dozens of packet types per state) | ~15–20 message types, designed for BedWars only |
| Legacy framing, obfuscation-era quirks, legacy fallbacks | Clean versioned binary framing from day 1 |
| Vanilla chunk/Anvil formats bit-exact | Our own compact world format (+ importer for maps) |
| Mojang auth (or offline-mode quirks) | Username-first arcade auth; accounts later |
| Vanilla assets (we can't ship theirs) | **Our own art** — CC0/commissioned/procedural |
| Every mechanic (mobs, redstone, portals…) | Players, blocks, items, projectiles. That's the entity list. |

The client and server **share** the protocol and world-format crates — one source of truth, no drift.

---

## 2. System architecture

```
┌───────────────────────────────────┐          ┌───────────────────────────────────┐
│ melon-client (Rust, wgpu)         │          │ melon-server (Rust, tokio)        │
│                                   │          │                                   │
│  wgpu renderer                    │          │  networking (TLS TCP listener)    │
│   texture atlas · section meshes  │          │        │                          │
│   frustum culling · sky/fog       │  melon   │  runtime — 20 TPS fixed-step sim  │
│  input (mouse-look, WASD, hotbar) │◄─proto──▶│   ONE actor per match             │
│  UI (egui): chat, score, hotbar   │  (TLS    │   tick budget · entity bucketing  │
│  movement + prediction            │   TCP)   │  world · entities · combat        │
│  audio (later)                    │          │  matchmaking · BedWars engine     │
└───────────────────────────────────┘          └───────────────────────────────────┘
              ▲                                              ▲
              └────────────── melon-proto ───────────────────┘
                     shared: message types, framing, encoding
              ┌────────────── melon-world ───────────────────┐
                     shared: block registry, chunks, arenas
```

**One trick worth its weight:** the client binary takes a `--headless` flag (no window, scripted input). The same codebase is then the **bot harness** for load tests and CI — 200 bots joining and fighting with zero extra tooling.

## 3. Workspace crate map

| Crate | Owns | Key dependencies |
|---|---|---|
| `melon-proto` *(shared)* | Handshake + version negotiation, message enums (both directions), compact binary codec, framing, TLS setup | `tokio-util`, `rustls`, `serde`, `bincode` (or hand-rolled) |
| `melon-world` *(shared)* | Block registry (IDs, metadata, hardness), chunk storage (16³ sections, flat `Vec`), arena format + vanilla-map importer | `thiserror`, `byteorder` |
| `melon-client` | wgpu renderer, atlas + greedy section meshing, input, egui UI, movement/prediction, audio (later) | `wgpu`, `winit`, `egui`, `kira` (later) |
| `melon-server` | Listener, match actors, tick loop, entity sim, combat/KB, matchmaking, BedWars FSM | `tokio`, `parking_lot`, `crossbeam` |
| `melon-tools` | Arena converter, benchmark bots, admin CLI | — |

**Commitments carried over from the density thesis (unchanged):** one actor per match (single-threaded sim, zero locks), allocation discipline on hot paths, zero unsafe by default.

## 4. Custom protocol — design principles

1. **Versioned from day 1.** Handshake exchanges `protocol_version` + build info; the codec refuses mismatched versions loudly. Retrofitting versioning is miserable — we start with it.
2. **Compact binary, no JSON on the hot path.** VarInt-encoded lengths/counts, bincode-style typed structs; text only for chat/MOTD.
3. **Message set (v0 draft, ~18 messages):**
   - `Handshake` · `Ping/Pong` · `Auth { username }` · `AuthResult`
   - `World` · `ChunkData { sectioned blocks }` · `BlockChange { batch }`
   - `PlayerJoin/Leave` · `PlayerMove { pos, look }` · `PlayerLook`
   - `BlockPlace/Break` · `HeldItem` · `Inventory`
   - `Swing` · `Hit { target }` · `Velocity { knockback }`
   - `Chat` · `Scoreboard` · `MatchState { phase, teams }` · `MatchEvent`
4. **Transport:** TCP + rustls TLS (simple, fast, fine for BedWars latency). QUIC noted as a future experiment, not the default.
5. **Server trust model:** client-authoritative movement **with clamps** (speed/teleport caps, rate limits) from the first multiplayer milestone; server-side hit validation when competitive integrity matters (M6).

## 5. Client architecture (the big lift — scoped honestly)

| Subsystem | Approach |
|---|---|
| Rendering | `wgpu` + `winit`. Chunk sections (16³) meshed **greedy**, one vertex buffer per section, redraw only on dirty; texture atlas + nearest-neighbor filtering; frustum culling; simple sky gradient + distance fog |
| First render target | A flat world at 60+ FPS on a laptop iGPU — meshing benchmark lands in **M2**, not at the end |
| UI | `egui` (chat window, scoreboard, hotbar, disconnect/menu screens) |
| Movement | Client-side simulation (1.8-feel constants from the reference analysis: gravity, jump arcs, sprint), mouse-look via raw input |
| Netcode | Client-authoritative + clamps first; add reconciliation only when cheating becomes real |
| Assets | **Own art only**: procedural texture atlas first (script-generated), CC0/CC-BY pack or commissioned art for the polish pass. Never Mojang assets |
| Audio | `kira` in the polish phase (steps, block sounds, hit sounds) |

## 6. Server architecture (mostly unchanged from the engine thesis)

- 20 TPS fixed-step; per-match actors; entity bucketing by chunk for O(local) updates.
- World source: procedural flat/skyblock grids first; **melon arena format** (own container: blocks + metadata + spawn points + team data) with an importer for vanilla-built maps (`.schematic`/`.mca` read-only) so mappers can still build in vanilla.
- BedWars entity set (hard scope): players, dropped items, arrows/projectiles, fireworks. No mobs, no redstone, no villagers — shop is a UI screen.
- Matchmaking: queue → arena allocation → match lifecycle FSM → result reporting.

## 7. Milestone ladder

| # | Milestone | Ships | Acceptance test |
|---|---|---|---|
| M0 | Workspace & CI | 5 crates as stubs, fmt/clippy/test in CI | green CI |
| M1 | **Protocol v0 vertical slice** | handshake/versioning/ping/auth + echo server | CLI client and CLI server talk the real protocol |
| M2 | **Client renders a world** | wgpu window, flat-world meshing, fly camera, atlas benchmark | 60+ FPS fly-cam over a flat world on a laptop iGPU |
| M3 | **Server-hosted world** | ChunkData over the wire, block place/break synced | Two windows edit the same server-side world |
| M4 | **Presence & chat** | PlayerJoin/Leave/Move, nametags, chat UI | See another player walk; chat works |
| M5 | **Player model & inventory** | Humanoid model/animation, hotbar, held item | Players look like players; hotbar works |
| M6 | **Combat** | Swing/hit/velocity, 1.8 knockback constants, clamps | A duel feels right; hits are server-validated |
| M7 | **Arena format & maps** | melon arena container + vanilla map importer | A real BedWars map built in vanilla loads and renders |
| M8 | **BedWars vertical slice** | Teams, beds, generators, shop UI, win condition | A full match completes between two clients |
| M9 | **Polish** | Audio, particles, art pass, settings, reconnect | Feels like a game, not a demo |
| M10 | **Vanilla bridge** | Gateway translating Protocol 47 ⇄ melon-proto | A vanilla 1.8.9 client joins a MELON match |
| M11 | **Density & benchmarks** | Multi-match runtime, public suite | Publish players/core + matches/machine numbers |

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Client rendering perf (meshing stalls, draw calls) | Greedy meshing + dirty-section redraws; the **M2 benchmark gate** refuses to let perf debt accumulate |
| Art/asset gap (no textures we own) | Procedural atlas from M2; commission/CC0 pack in M9; never Mojang assets |
| Custom protocol drift between client/server | Shared `melon-proto` crate is the only codec; version negotiation rejects mismatches |
| Predictive-movement complexity | Start client-authoritative + clamps; reconciliation is an upgrade, not a prerequisite |
| Scope creep (mobs, redstone, survival systems) | Entity set is a hard list; new entity = needs a BedWars justification |
| "When can vanilla players join?" | Honest answer: M10. The bridge is scheduled, not hand-waved |

## 9. Immediate next steps (this week)

1. **M0:** Cargo workspace with `melon-proto`, `melon-world`, `melon-client`, `melon-server`, `melon-tools` + CI.
2. **M1:** handshake/version negotiation/ping/auth messages + echo round-trip between two CLI binaries — the protocol spine, ~1–2 days.
3. **M2 kickoff:** wgpu window + procedural texture atlas + flat-world greedy mesher — the first "it renders" moment.

---

*Clean-room commitment (still in force): no Mojang code, assets, or data files are used, copied, or distributed. The vanilla 1.8.9 jar stays a private reference artifact; the future bridge speaks only the public Protocol 47 spec.*
