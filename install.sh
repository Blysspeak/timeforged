#!/usr/bin/env bash
set -euo pipefail

# ── TimeForged Installer ──
# Full out-of-the-box setup: local daemon, remote sync, Claude Code hooks, public card
# Usage: git clone <repo> && cd timeforged && bash install.sh

BOLD='\033[1m'
DIM='\033[2m'
ORANGE='\033[38;5;214m'
GREEN='\033[32m'
RED='\033[31m'
YELLOW='\033[33m'
RESET='\033[0m'

info()  { echo -e "${ORANGE}▸${RESET} $1"; }
ok()    { echo -e "${GREEN}✓${RESET} $1"; }
warn()  { echo -e "${YELLOW}!${RESET} $1"; }
fail()  { echo -e "${RED}✗${RESET} $1"; exit 1; }
header() { echo -e "\n${BOLD}${ORANGE}$1${RESET}"; }

echo ""
echo -e "${BOLD}${ORANGE}  ╔══════════════════════════════════════╗${RESET}"
echo -e "${BOLD}${ORANGE}  ║         TimeForged Installer         ║${RESET}"
echo -e "${BOLD}${ORANGE}  ║     self-hosted time tracker          ║${RESET}"
echo -e "${BOLD}${ORANGE}  ╚══════════════════════════════════════╝${RESET}"
echo ""

# ── 1. Check prerequisites ──
header "Checking prerequisites..."

command -v cargo >/dev/null 2>&1 || fail "Rust/Cargo not found. Install from https://rustup.rs"
ok "Rust $(rustc --version | awk '{print $2}')"

command -v npm >/dev/null 2>&1 || fail "Node.js/npm not found. Install from https://nodejs.org"
ok "Node $(node --version) / npm $(npm --version)"

HAS_JQ=false
if command -v jq >/dev/null 2>&1; then
    HAS_JQ=true
    ok "jq $(jq --version)"
else
    warn "jq not found — Claude Code hooks will be skipped"
fi

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

# ── 2. Build web dashboard ──
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

# ── 3. Build Rust binaries ──
header "Building Rust binaries..."

info "Compiling (this may take a few minutes on first run)..."
cargo build --release 2>&1 | grep -E "Compiling|Finished" || true
ok "Binaries built"

DAEMON_BIN="$PROJECT_DIR/target/release/timeforged"
CLI_BIN="$PROJECT_DIR/target/release/tf"

[[ -f "$DAEMON_BIN" ]] || fail "Daemon binary not found"
[[ -f "$CLI_BIN" ]] || fail "CLI binary not found"
ok "timeforged daemon: $DAEMON_BIN"
ok "tf CLI:            $CLI_BIN"

# ── 4. Install to ~/.local/bin ──
INSTALL_DIR="$HOME/.local/bin"
header "Installing binaries..."

mkdir -p "$INSTALL_DIR"
/usr/bin/cp "$DAEMON_BIN" "$INSTALL_DIR/timeforged"
/usr/bin/cp "$CLI_BIN" "$INSTALL_DIR/tf"
ok "Installed to $INSTALL_DIR/"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "  ${DIM}Add to your shell profile:${RESET}"
    echo -e "  ${DIM}  export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
    echo ""
fi

# ── 5. First run — start daemon, capture API key ──
header "Starting TimeForged daemon..."

pkill -f "$INSTALL_DIR/timeforged" 2>/dev/null || true
sleep 1

DAEMON_LOG=$(mktemp)
"$INSTALL_DIR/timeforged" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!
sleep 2

if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
    cat "$DAEMON_LOG"
    fail "Daemon failed to start"
fi
ok "Daemon running (PID: $DAEMON_PID)"

API_KEY=""
if grep -q "API key:" "$DAEMON_LOG"; then
    API_KEY=$(grep "API key:" "$DAEMON_LOG" | grep -oP 'tf_\w+' || true)
fi

CLI_CONFIG="$HOME/.config/timeforged/cli.toml"
if [[ -z "$API_KEY" && -f "$CLI_CONFIG" ]]; then
    EXISTING_KEY=$(grep -oP 'api_key\s*=\s*"\K[^"]+' "$CLI_CONFIG" 2>/dev/null || true)
    if [[ -n "$EXISTING_KEY" ]]; then
        API_KEY="$EXISTING_KEY"
    fi
fi

if [[ -n "$API_KEY" ]]; then
    mkdir -p "$(dirname "$CLI_CONFIG")"
    cat > "$CLI_CONFIG" <<EOF
server_url = "http://127.0.0.1:6175"
api_key = "$API_KEY"
EOF
    ok "CLI config saved to $CLI_CONFIG"
fi

HEALTH=$(curl -sf http://127.0.0.1:6175/health 2>/dev/null || echo "")
if [[ -z "$HEALTH" ]]; then
    fail "Daemon not responding on port 6175"
fi
ok "Health check passed"

rm -f "$DAEMON_LOG"

# ── 6. Install systemd user service (daemon) ──
USE_SYSTEMD=false
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
    fi
fi

# ── 7. Remote registration & sync ──
header "Setting up remote sync..."

DEFAULT_REMOTE="https://timeforged.blysspeak.space"
TF_USERNAME=""
REMOTE_KEY=""
REMOTE_URL=""
REMOTE_OK=false

read -rp "$(echo -e "${ORANGE}▸${RESET} Remote server [$DEFAULT_REMOTE]: ")" REMOTE_URL
REMOTE_URL="${REMOTE_URL:-$DEFAULT_REMOTE}"

# Check if remote is reachable
if curl -sf --max-time 5 "$REMOTE_URL/health" >/dev/null 2>&1; then
    ok "Remote server reachable"

    while true; do
        read -rp "$(echo -e "${ORANGE}▸${RESET} Choose a username: ")" TF_USERNAME
        if [[ -z "$TF_USERNAME" ]]; then
            warn "Username cannot be empty"
            continue
        fi

        info "Registering as $TF_USERNAME on $REMOTE_URL..."
        REGISTER_OUTPUT=$("$INSTALL_DIR/tf" register "$TF_USERNAME" --remote "$REMOTE_URL" 2>&1) && REG_RC=0 || REG_RC=$?

        if [[ $REG_RC -eq 0 ]]; then
            REMOTE_KEY=$(echo "$REGISTER_OUTPUT" | grep -oP 'tf_\w+' || true)
            if [[ -n "$REMOTE_KEY" ]]; then
                ok "Registered as $TF_USERNAME"
                REMOTE_OK=true
                break
            else
                warn "Registration succeeded but could not extract API key"
                echo "$REGISTER_OUTPUT"
                break
            fi
        else
            # Check for "already exists" / 409 conflict
            if echo "$REGISTER_OUTPUT" | grep -qi "taken\|exists\|conflict\|409"; then
                warn "Username '$TF_USERNAME' is already taken. Try another."
                continue
            else
                warn "Registration failed: $REGISTER_OUTPUT"
                break
            fi
        fi
    done
else
    warn "Remote server unreachable at $REMOTE_URL — skipping remote setup"
    info "Local tracking will still work. Run remote setup later with:"
    echo -e "  ${DIM}tf register <username> --remote $REMOTE_URL${RESET}"
fi

# Update cli.toml with remote config
if [[ "$REMOTE_OK" == "true" && -n "$API_KEY" ]]; then
    cat > "$CLI_CONFIG" <<EOF
server_url = "http://127.0.0.1:6175"
api_key = "$API_KEY"
remote_url = "$REMOTE_URL"
remote_key = "$REMOTE_KEY"
EOF
    ok "CLI config updated with remote credentials"

    # Enable public profile
    info "Enabling public profile..."
    "$INSTALL_DIR/tf" profile --public 2>&1 || warn "Could not enable public profile"

    # Initial sync
    info "Running initial sync..."
    "$INSTALL_DIR/tf" sync 2>&1 || warn "Initial sync had issues (this is normal for a fresh install)"
fi

# ── 8. Auto-sync timer (every 15 min) ──
if [[ "$USE_SYSTEMD" == "true" && "$REMOTE_OK" == "true" ]]; then
    header "Installing auto-sync timer..."

    cat > "$SYSTEMD_DIR/tf-sync.service" <<EOF
[Unit]
Description=TimeForged — sync local events to remote
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=$INSTALL_DIR/tf sync
Environment=HOME=$HOME
EOF

    cat > "$SYSTEMD_DIR/tf-sync.timer" <<EOF
[Unit]
Description=TimeForged — auto-sync every 15 minutes

[Timer]
OnCalendar=*:0/15
Persistent=true
RandomizedDelaySec=60

[Install]
WantedBy=timers.target
EOF

    systemctl --user daemon-reload
    systemctl --user enable --now tf-sync.timer
    ok "Auto-sync timer installed (every 15 min)"
fi

# ── 9. Claude Code hooks (auto-install, no prompt) ──
CLAUDE_DIR="$HOME/.claude"
CLAUDE_HOOKS_INSTALLED=false
if [[ -d "$CLAUDE_DIR" && "$HAS_JQ" == "true" ]]; then
    header "Configuring Claude Code integration..."

    mkdir -p "$CLAUDE_DIR/hooks"
    /usr/bin/cp "$PROJECT_DIR/contrib/claude-code/timeforged-heartbeat.sh" "$CLAUDE_DIR/hooks/timeforged-heartbeat.sh"
    chmod +x "$CLAUDE_DIR/hooks/timeforged-heartbeat.sh"
    ok "Hook script installed"

    CLAUDE_SETTINGS="$CLAUDE_DIR/settings.json"
    HOOK_CMD="$CLAUDE_DIR/hooks/timeforged-heartbeat.sh"
    HOOK_CONFIG=$(jq -n --arg cmd "$HOOK_CMD" '{
      hooks: {
        UserPromptSubmit: [{ matcher: "", hooks: [{ type: "command", command: $cmd, timeout: 5 }] }],
        PostToolUse:      [{ matcher: "", hooks: [{ type: "command", command: $cmd, timeout: 5 }] }],
        Stop:             [{ matcher: "", hooks: [{ type: "command", command: $cmd, timeout: 5 }] }]
      }
    }')

    if [[ -f "$CLAUDE_SETTINGS" ]]; then
        TMP=$(mktemp)
        jq --argjson hooks "$(echo "$HOOK_CONFIG" | jq '.hooks')" '.hooks = (.hooks // {}) * $hooks' "$CLAUDE_SETTINGS" > "$TMP"
        mv "$TMP" "$CLAUDE_SETTINGS"
    else
        echo "$HOOK_CONFIG" > "$CLAUDE_SETTINGS"
    fi
    ok "Claude Code hooks configured"
    CLAUDE_HOOKS_INSTALLED=true
elif [[ -d "$CLAUDE_DIR" && "$HAS_JQ" == "false" ]]; then
    warn "Skipping Claude Code hooks — install jq and re-run"
fi

# ── 10. Waybar module (if waybar installed) ──
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

echo -e "  ${BOLD}Dashboard:${RESET}   ${GREEN}http://127.0.0.1:6175${RESET}"

if [[ -n "$API_KEY" ]]; then
echo -e "  ${BOLD}API Key:${RESET}     ${ORANGE}${API_KEY}${RESET}"
fi

if [[ "$REMOTE_OK" == "true" ]]; then
echo -e "  ${BOLD}Card URL:${RESET}    ${GREEN}${REMOTE_URL%/}/github/timeforged/${TF_USERNAME}.svg${RESET}"
echo -e "  ${BOLD}Auto-sync:${RESET}   every 15 min (tf-sync.timer)"
fi

if [[ "$CLAUDE_HOOKS_INSTALLED" == "true" ]]; then
echo -e "  ${BOLD}Claude Code:${RESET} hooks installed"
fi

echo ""
echo -e "  ${DIM}Config: ~/.config/timeforged/cli.toml${RESET}"
echo ""

if [[ "$REMOTE_OK" == "true" ]]; then
echo -e "  ${BOLD}Add to your GitHub README:${RESET}"
echo -e "    ${GREEN}<img src=\"${REMOTE_URL%/}/github/timeforged/${TF_USERNAME}.svg\" />${RESET}"
echo ""
fi

echo -e "  ${BOLD}Quick start:${RESET}"
echo -e "    ${GREEN}tf init ~/projects${RESET}  — start tracking a directory"
echo -e "    ${DIM}tf today${RESET}            — today's summary"
echo -e "    ${DIM}tf status${RESET}           — daemon status"
echo -e "    ${DIM}tf sync${RESET}             — manual sync to remote"
echo ""

if [[ "$USE_SYSTEMD" == "true" ]]; then
echo -e "  ${BOLD}Systemd:${RESET}"
echo -e "    ${DIM}systemctl --user status timeforged${RESET}    — daemon"
echo -e "    ${DIM}systemctl --user status tf-sync.timer${RESET} — auto-sync"
echo -e "    ${DIM}journalctl --user -u timeforged -f${RESET}    — logs"
else
echo -e "  ${BOLD}Daemon:${RESET}"
echo -e "    ${DIM}timeforged${RESET}          — start (foreground)"
fi
echo ""
