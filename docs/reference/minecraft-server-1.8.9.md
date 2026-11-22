# Reference Analysis — Vanilla Minecraft Server 1.8.9 (`server.jar`)

> Status: **Reference artifact** · Analyzed: 2026-11-22 · Analysis type: static (no execution, no decompilation)
>
> **Note on direction:** MELON is a custom-protocol platform (own client + own server), so this analysis is *background* — it informs 1.8-feel gameplay constants (§6), the translation-key surface, and the future vanilla-bridge phase (M10 in the [engine plan](../engine-plan.md)). It is not a blueprint we implement against.

## 1. Identity

| Property | Value |
|---|---|
| Artifact | Vanilla Minecraft Java Edition **1.8.9** dedicated server jar |
| Original filename | `server (2).jar` (user Downloads) |
| In-repo copy | [`reference/minecraft-server-1.8.9.jar`](../../reference/minecraft-server-1.8.9.jar) |
| Size | 8,320,755 bytes (8.3 MB) |
| SHA-256 | `c18e4245073aaff580eb7359902f0251436568b1647a9e443a924cdb73fa8312` |
| Entry timestamps | 2015-12-09 10:24 (matches the 1.8.9 release date: December 9, 2015) |
| Main class | `net.minecraft.server.MinecraftServer` |
| Packaging | Classic self-contained "fat" jar (no modern bundler format, no `version.json`) |
| Code identity state | **Obfuscated** (single-letter class names at jar root) |

**How identity was established** — without executing anything: `Main-Class: net.minecraft.server.MinecraftServer` + 2015-12-09 timestamps + pre-mappings-era obfuscated class layout uniquely identifies the vanilla 1.8.9 server. (The larger `server.jar` in Downloads, 60 MB, is a *modern* Mojang bundler-format jar — `net.minecraft.bundler.Main`, `Bundler-Format: 1.0` — unrelated to 1.8.9 and not used by this project.)

## 2. Contents overview

**5,925 zip entries** (16,737,642 bytes uncompressed): **5,599 classes** + ~170 resources.

| Package | Classes | What it is | State |
|---|---:|---|---|
| *(jar root)* | 1,655 | **The game engine itself** — world, entities, protocol, ticking (`a.class`, `aa.class`, …) | Obfuscated |
| `com/google/common` | 1,683 | Google **Guava** (collections, caching, concurrency utilities) | Readable |
| `io/netty` | 1,094 | **Netty 4.0.23.Final** async TCP framework (incl. `libnetty-transport-native-epoll.so` for Linux) | Readable |
| `org/apache/logging` | 505 | Apache **Log4j 2.x** (API 2.0.0) | Readable |
| `org/apache/commons` | 412 | Apache Commons **Codec** + **IO** | Readable |
| `com/google/gson` | 153 | **Gson** JSON library | Readable |
| `com/mojang/authlib` | 55 | **Mojang Authlib** — profiles, Yggdrasil/session auth (see §4) | Readable |
| `javax/annotation` | 34 | JSR-305 nullness annotations | Readable |
| `net/minecraft/server` | 5 | `MinecraftServer` + 4 inner classes — the only readable `net.minecraft` code (entry point, 31 KB) | Readable |

**Notable resources**

| Resource | Significance |
|---|---|
| `assets/minecraft/lang/en_US.lang` | 2,553 translation keys — every chat/GUI string key the 1.8.9 client resolves |
| `META-INF/io.netty.versions.properties` | Confirms Netty 4.0.23.Final |
| `META-INF/native/libnetty-transport-native-epoll.so` | Native epoll transport (Linux) |
| `Log4j-*.xsd/.dtd`, `log4j-provider.properties` | Logging config surface |
| `META-INF/INDEX.LIST` | Fat-jar index |

**What is *not* in the jar:** world data, textures/assets (server-side doesn't ship them), datapacks (1.8 predates them), external libraries as separate files — everything needed to run is inside the fat jar.

## 3. What this tells us about 1.8.9's runtime shape

The jar's composition is effectively a dependency manifest for our Rust reimplementation:

| 1.8.9 (Java) | Role | MELON (Rust) equivalent |
|---|---|---|
| Netty 4.0 + epoll | Async TCP, length-prefixed frames | `tokio` + `tokio-util` codec |
| Guava | Collections, caches, `HashBiMap` etc. | `std` + `parking_lot`, `rustc-hash`, `flume`/`crossbeam` |
| Gson | JSON (status response, chat components) | `serde` + `serde_json` |
| Log4j 2 | Structured logging | `tracing` + `tracing-subscriber` |
| Commons Codec/IO | Hex, Base64, IO helpers | `hex`, `base64`, `flate2`, `std::io` |
| Authlib | Profile/session auth | Our own minimal `auth` crate (§4) |
| Obfuscated engine | Game simulation | The thing we build, clean-room |

## 4. The authentication model (from readable Authlib)

Authlib being unobfuscated maps the entire **online-mode** flow the server must implement:

1. Client → server: *Login Start* (username).
2. Server → Mojang session server: join request for that player.
3. Server → client: *Encryption Request* (RSA-1024 public key + verify token).
4. Client → server: *Encryption Response* (shared secret + verify token, both RSA-encrypted).
5. Server → Mojang session server: *hasJoined* check — on success the server receives the canonical `GameProfile` (UUID + skin properties).
6. Server → client: *Login Success* (UUID + username). From here the stream is AES-CFB8 encrypted and (optionally) zlib-compressed per 1.8 rules.

For an **arcade network**, offline-mode (no Mojang session server) or proxy-managed auth (Bungee-style) is the pragmatic path; full Yggdrasil integration stays behind a feature flag. Authlib also defines `Property`/`PropertyMap` (skin/textures payloads) which the status/login responses may embed.

## 5. Clean-room policy & legal notes

- The jar is **Mojang's proprietary code**. It is checked in *only* as a reference artifact for interoperability documentation of the 1.8.9 protocol surface.
- **We do not decompile, disassemble, execute, or redistribute its code.** Nothing from the jar (code, assets, strings, data files) is copied into MELON.
- MELON is implemented **clean-room**: from public protocol documentation ([wiki.vg, protocol version 47](https://wiki.vg/Protocol)), the public NBT / Anvil / region-format specs, and our own protocol captures (see the engine plan's testing strategy).
- Community mapping vocabularies (MCP 1.8.9 names, Bukkit's `v1_8_R3` package names) may be consulted **as naming inspiration only** — never as source.
- ⚠️ **Repo-hygiene note:** redistributing Mojang's jar in a public repository is legally dubious even as an artifact. If this repo goes public, consider removing `reference/minecraft-server-1.8.9.jar` from git and keeping it out-of-band (e.g., `.gitignore` + a download script pinned to the SHA-256 above).

## 6. Quick facts worth remembering

- Largest obfuscated classes are 56 KB / 37 KB / 35 KB (`adm.class`, `pk.class`, `afh.class`) — consistent with the biggest simulation subsystems (world/block registration scale).
- 1.8.9 ships **before** the 1.9 combat rework — the attack-cooldown era; PvP feel is swing-rate-driven. This is the exact combat model BedWars communities prefer and our `combat` crate must reproduce.
- The server list ping (`Status` state) is JSON over TCP with a legacy 0xFE ping fallback for ancient clients — irrelevant for 1.8.9 clients but worth knowing for proxy compat.
