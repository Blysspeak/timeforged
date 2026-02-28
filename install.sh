#!/usr/bin/env bash
set -euo pipefail

# ── TimeForged Installer ──
# Self-hosted time tracking daemon + web dashboard
# Usage: curl -sSL <repo>/install.sh | bash
#   or:  git clone <repo> && cd timeforged && bash install.sh

BOLD='\033[1m'
DIM='\033[2m'
ORANGE='\033[38;5;214m'
GREEN='\033[32m'
RED='\033[31m'
RESET='\033[0m'

info()  { echo -e "${ORANGE}▸${RESET} $1"; }
ok()    { echo -e "${GREEN}✓${RESET} $1"; }
fail()  { echo -e "${RED}✗${RESET} $1"; exit 1; }
header() { echo -e "\n${BOLD}${ORANGE}$1${RESET}"; }

echo ""
echo -e "${BOLD}${ORANGE}  ╔══════════════════════════════════════╗${RESET}"
echo -e "${BOLD}${ORANGE}  ║         TimeForged Installer         ║${RESET}"
echo -e "${BOLD}${ORANGE}  ║     self-hosted time tracker          ║${RESET}"
echo -e "${BOLD}${ORANGE}  ╚══════════════════════════════════════╝${RESET}"
echo ""

# ── Check prerequisites ──
header "Checking prerequisites..."

command -v cargo >/dev/null 2>&1 || fail "Rust/Cargo not found. Install from https://rustup.rs"
ok "Rust $(rustc --version | awk '{print $2}')"

command -v npm >/dev/null 2>&1 || fail "Node.js/npm not found. Install from https://nodejs.org"
ok "Node $(node --version) / npm $(npm --version)"

# ── Determine project root ──
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/Cargo.toml" ]]; then
    PROJECT_DIR="$SCRIPT_DIR"
else
    PROJECT_DIR="$(pwd)"
fi

[[ -f "$PROJECT_DIR/Cargo.toml" ]] || fail "Not in TimeForged project root. Run from the repo directory."
cd "$PROJECT_DIR"
ok "Project root: $PROJECT_DIR"

# ── Build web dashboard ──
header "Building web dashboard..."

WEB_DIR="$PROJECT_DIR/crates/timeforged/web"
if [[ ! -d "$WEB_DIR" ]]; then
    fail "Web directory not found at $WEB_DIR"
fi

cd "$WEB_DIR"

info "Installing npm dependencies..."
npm install --silent 2>&1 | tail -1
ok "Dependencies installed"

info "Building Vue app..."
npx vite build --logLevel error 2>&1
ok "Dashboard built"

cd "$PROJECT_DIR"

# ── Build Rust binaries ──
header "Building Rust binaries..."

info "Compiling (this may take a few minutes on first run)..."
cargo build --release 2>&1 | grep -E "Compiling|Finished" || true
ok "Binaries built"

# Check binaries exist
DAEMON_BIN="$PROJECT_DIR/target/release/timeforged"
CLI_BIN="$PROJECT_DIR/target/release/tf"

[[ -f "$DAEMON_BIN" ]] || fail "Daemon binary not found"
[[ -f "$CLI_BIN" ]] || fail "CLI binary not found"
ok "timeforged daemon: $DAEMON_BIN"
ok "tf CLI:            $CLI_BIN"

# ── Install to ~/.local/bin (optional) ──
INSTALL_DIR="$HOME/.local/bin"
header "Installing binaries..."

mkdir -p "$INSTALL_DIR"
cp "$DAEMON_BIN" "$INSTALL_DIR/timeforged"
cp "$CLI_BIN" "$INSTALL_DIR/tf"
ok "Installed to $INSTALL_DIR/"

# Check PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "  ${DIM}Add to your shell profile:${RESET}"
    echo -e "  ${DIM}  export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
    echo ""
fi

# ── First run — start daemon, capture API key ──
header "Starting TimeForged daemon..."

# Kill any existing instance
pkill -f "$INSTALL_DIR/timeforged" 2>/dev/null || true
sleep 1

# Start daemon and capture output
DAEMON_LOG=$(mktemp)
"$INSTALL_DIR/timeforged" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!
sleep 2

# Check it's running
if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    cat "$DAEMON_LOG"
    fail "Daemon failed to start"
fi
ok "Daemon running (PID: $DAEMON_PID)"

# Extract API key if first run
API_KEY=""
if grep -q "API key:" "$DAEMON_LOG"; then
    API_KEY=$(grep "API key:" "$DAEMON_LOG" | grep -oP 'tf_\w+' || true)
fi

# Also check if CLI config already has a key
CLI_CONFIG="$HOME/.config/timeforged/cli.toml"
if [[ -z "$API_KEY" && -f "$CLI_CONFIG" ]]; then
    EXISTING_KEY=$(grep -oP 'api_key\s*=\s*"\K[^"]+' "$CLI_CONFIG" 2>/dev/null || true)
    if [[ -n "$EXISTING_KEY" ]]; then
        API_KEY="$EXISTING_KEY"
    fi
fi

# Save API key to CLI config
if [[ -n "$API_KEY" ]]; then
    mkdir -p "$(dirname "$CLI_CONFIG")"
    cat > "$CLI_CONFIG" <<EOF
server_url = "http://127.0.0.1:6175"
api_key = "$API_KEY"
EOF
    ok "CLI config saved to $CLI_CONFIG"
fi

# Verify health
HEALTH=$(curl -sf http://127.0.0.1:6175/health 2>/dev/null || echo "")
if [[ -z "$HEALTH" ]]; then
    fail "Daemon not responding on port 6175"
fi
ok "Health check passed"

rm -f "$DAEMON_LOG"

# ── Install systemd user service (if systemd is available) ──
if command -v systemctl >/dev/null 2>&1; then
    header "Setting up systemd user service..."

    SYSTEMD_DIR="$HOME/.config/systemd/user"
    mkdir -p "$SYSTEMD_DIR"
    cat > "$SYSTEMD_DIR/timeforged.service" <<EOF
[Unit]
Description=TimeForged — self-hosted time tracking daemon
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/timeforged
Restart=on-failure
RestartSec=5
Environment=TF_LOG_LEVEL=info

[Install]
WantedBy=default.target
EOF

    # Stop the daemon we started manually — systemd will manage it now
    kill "$DAEMON_PID" 2>/dev/null || true
    sleep 1

    systemctl --user daemon-reload
    systemctl --user enable timeforged.service
    systemctl --user start timeforged.service
    sleep 2

    if systemctl --user is-active --quiet timeforged.service; then
        ok "Systemd service installed and running"
        DAEMON_PID=$(systemctl --user show timeforged.service -p MainPID --value)
        USE_SYSTEMD=true
    else
        info "Systemd service created but failed to start, running manually"
        "$INSTALL_DIR/timeforged" > /dev/null 2>&1 &
        DAEMON_PID=$!
        sleep 2
        USE_SYSTEMD=false
    fi
else
    USE_SYSTEMD=false
fi

# ── Install Waybar module (if waybar is present) ──
if command -v waybar >/dev/null 2>&1; then
    header "Installing Waybar module..."
    WAYBAR_SCRIPT="$PROJECT_DIR/contrib/waybar/install.sh"
    if [[ -f "$WAYBAR_SCRIPT" ]]; then
        bash "$WAYBAR_SCRIPT"
    else
        info "Waybar installer not found, skipping"
    fi
fi

# ── Summary ──
echo ""
echo -e "${BOLD}${ORANGE}  ╔══════════════════════════════════════╗${RESET}"
echo -e "${BOLD}${ORANGE}  ║       Installation Complete!         ║${RESET}"
echo -e "${BOLD}${ORANGE}  ╚══════════════════════════════════════╝${RESET}"
echo ""

echo -e "  ${BOLD}Dashboard:${RESET}  ${GREEN}http://127.0.0.1:6175${RESET}"
echo ""

if [[ -n "$API_KEY" ]]; then
echo -e "  ${BOLD}API Key:${RESET}    ${ORANGE}${API_KEY}${RESET}"
echo -e "  ${DIM}(saved to ~/.config/timeforged/cli.toml)${RESET}"
echo ""
fi

echo -e "  ${BOLD}Quick start:${RESET}"
echo -e "    ${GREEN}tf init ~/projects${RESET}  — start tracking a directory"
echo -e "    ${DIM}tf today${RESET}            — today's summary"
echo -e "    ${DIM}tf list${RESET}             — show watched directories"
echo ""
echo -e "  ${BOLD}All commands:${RESET}"
echo -e "    ${DIM}tf init [path]${RESET}      — watch directory (default: cwd)"
echo -e "    ${DIM}tf unwatch <path>${RESET}   — stop watching"
echo -e "    ${DIM}tf status${RESET}           — daemon status"
echo -e "    ${DIM}tf today${RESET}            — today's summary"
echo -e "    ${DIM}tf report${RESET}           — detailed report"
echo -e "    ${DIM}tf send <file>${RESET}      — manual heartbeat"
echo ""

if [[ "$USE_SYSTEMD" == "true" ]]; then
echo -e "  ${BOLD}Daemon (systemd):${RESET}"
echo -e "    ${DIM}systemctl --user status timeforged${RESET}"
echo -e "    ${DIM}systemctl --user restart timeforged${RESET}"
echo -e "    ${DIM}journalctl --user -u timeforged -f${RESET}"
else
echo -e "  ${BOLD}Daemon:${RESET}"
echo -e "    ${DIM}timeforged${RESET}          — start (foreground)"
echo -e "    ${DIM}kill $DAEMON_PID${RESET}           — stop current"
fi
echo ""
echo -e "  ${DIM}Open ${GREEN}http://127.0.0.1:6175${RESET}${DIM} in your browser${RESET}"
echo ""
