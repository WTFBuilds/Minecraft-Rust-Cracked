#!/usr/bin/env bash
#
# MELON — one-command setup & launcher.
#
#   ./setup.sh              build + demo (server starts, headless client pings it)
#   ./setup.sh server       build + run the game server (foreground)
#   ./setup.sh client       build + open the game window (Phase 2 world)
#   ./setup.sh play         build + start server in background + open the window
#   ./setup.sh headless     build + run a headless protocol session
#   ./setup.sh demo         same as bare ./setup.sh
#   ./setup.sh screenshot   build + render one offscreen frame to a PNG
#   ./setup.sh test         fmt + clippy -D warnings + all tests
#   ./setup.sh -h           this help
#
# Env overrides:
#   MELON_HOST (127.0.0.1)  MELON_PORT (25777)  MELON_PLAYER (MelonPlayer)
#   MELON_NAME (MELON)      MELON_SERVER_LOG (/tmp/melon-server.log)
#
# Windows note: use WSL — the script is a plain POSIX-ish bash file.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

HOST="${MELON_HOST:-127.0.0.1}"
PORT="${MELON_PORT:-25777}"
PLAYER="${MELON_PLAYER:-MelonPlayer}"
SERVER_NAME="${MELON_NAME:-MELON}"
SERVER_LOG="${MELON_SERVER_LOG:-/tmp/melon-server.log}"
BIN=./target/debug

# --- pretty output -----------------------------------------------------------
if [ -t 1 ]; then
    G='\033[0;32m'; R='\033[0;31m'; B='\033[1m'; D='\033[2m'; O='\033[0m'
else
    G=''; R=''; B=''; D=''; O=''
fi
info() { printf "${G}▸${O} %s\n" "$*"; }
warn() { printf "${R}▸${O} %s\n" "$*"; }
head() { printf "\n${B}== %s ==${O}\n" "$*"; }

# --- toolchain ---------------------------------------------------------------
ensure_toolchain() {
    if ! command -v cargo >/dev/null 2>&1 && [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        info "Rust not found — installing via rustup (stable + clippy + rustfmt)…"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
            | sh -s -- -y --default-toolchain stable --profile minimal \
                      --component clippy,rustfmt
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    info "toolchain: $(cargo --version)"
}

build() {
    ensure_toolchain
    info "building workspace (debug)…"
    cargo build -q -p melon-server -p melon-client
    info "build ok"
}

# --- server lifecycle --------------------------------------------------------
server_port_busy() {
    if command -v lsof >/dev/null 2>&1; then
        lsof -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1
    else
        nc -z "$HOST" "$PORT" >/dev/null 2>&1
    fi
}

start_server() {
    if server_port_busy; then
        warn "port $PORT busy — stopping the old melon-server"
        pkill -f "melon-server --addr" 2>/dev/null || true
        sleep 0.5
    fi
    info "starting server on $HOST:$PORT (log: $SERVER_LOG)"
    ("$BIN/melon-server" --addr "$HOST:$PORT" --name "$SERVER_NAME" \
        --world-size "${MELON_WORLD_SIZE:-4}" > "$SERVER_LOG" 2>&1 &)
    # wait for the port to open (max ~10s)
    for _ in $(seq 1 40); do
        if nc -z "$HOST" "$PORT" >/dev/null 2>&1; then
            info "server is up"
            return 0
        fi
        sleep 0.25
    done
    warn "server did not open the port — log tail:"
    tail -5 "$SERVER_LOG" || true
    return 1
}

stop_server() {
    pkill -f "melon-server --addr" 2>/dev/null || true
}

# --- commands ----------------------------------------------------------------
cmd_demo() {
    build
    start_server
    head "DEMO — headless client vs server"
    set +e
    "$BIN/melon-client" --headless --host "$HOST" --port "$PORT" \
        --name "$PLAYER" --count 4
    rc=$?
    set -e
    stop_server
    if [ "$rc" -eq 0 ]; then info "demo passed ✔"; else warn "demo failed ✘"; fi
    exit "$rc"
}

cmd_server() {
    build
    head "SERVER (foreground, Ctrl-C to stop)"
    info "connect a client with: ./setup.sh headless   or   ./setup.sh client"
    exec "$BIN/melon-server" --addr "$HOST:$PORT" --name "$SERVER_NAME" --world-size "${MELON_WORLD_SIZE:-4}"
}

cmd_client() {
    build
    head "CLIENT WINDOW (ESC or close to quit)"
    exec "$BIN/melon-client"
}

cmd_play() {
    build
    start_server
    trap stop_server EXIT
    head "PLAY — server (background) + game window"
    info "server keeps running while the window is open (log: $SERVER_LOG)"
    set +e
    "$BIN/melon-client"
    set -e
    info "client closed — stopping server"
}

cmd_headless() {
    build
    head "HEADLESS CLIENT"
    exec "$BIN/melon-client" --headless --host "$HOST" --port "$PORT" \
        --name "$PLAYER" --count "${MELON_PINGS:-4}"
}

cmd_screenshot() {
    build
    local out="${1:-/tmp/melon-frame.png}"
    "$BIN/melon-client" --screenshot "$out"
    info "wrote $out"
}

cmd_test() {
    ensure_toolchain
    head "QUALITY GATES"
    cargo fmt --all -- --check
    info "fmt ok"
    cargo clippy --workspace --all-targets -- -D warnings -q
    info "clippy ok"
    cargo test --workspace
    info "tests ok"
}

cmd_help() {
    printf "${B}MELON${O} — lightweight Minecraft-style arcade platform\n\n"
    printf "${B}Usage:${O} ./setup.sh [command]\n\n"
    printf "${B}Commands:${O}\n"
    printf "  ${G}(none) | demo${O}   build + start server + headless client demo\n"
    printf "  ${G}server${O}          build + run the game server (foreground)\n"
    printf "  ${G}client${O}          build + open the game window\n"
    printf "  ${G}play${O}            server in background + game window (auto-cleanup)\n"
    printf "  ${G}headless${O}        protocol session on stdout (bot/CI mode)\n"
    printf "  ${G}screenshot${O}      offscreen render to PNG (default /tmp/melon-frame.png)\n"
    printf "  ${G}test${O}            fmt + clippy -D warnings + all tests\n"
    printf "  ${G}-h, help${O}        this help\n\n"
    printf "${B}Env:${O} MELON_HOST MELON_PORT MELON_PLAYER MELON_NAME MELON_WORLD_SIZE MELON_SERVER_LOG MELON_PINGS\n"
    printf "${D}Docs: docs/engine-plan.md · plan.md · docs/devlog.md${O}\n"
}

case "${1:-demo}" in
    demo)        cmd_demo ;;
    server)      cmd_server ;;
    client)      cmd_client ;;
    play)        cmd_play ;;
    headless)    cmd_headless ;;
    screenshot)  shift; cmd_screenshot "${1:-}" ;;
    test)        cmd_test ;;
    -h|--help|help) cmd_help ;;
    *) warn "unknown command: $1"; cmd_help; exit 2 ;;
esac
