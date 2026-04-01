#!/usr/bin/env bash
set -euo pipefail

# ── TimeForged Installer ──
# Fully autonomous: installs all dependencies, builds, configures, starts.
# Usage: git clone <repo> && cd timeforged && bash install.sh [linux|windows]

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

# ── Detect platform ──
detect_platform() {
    case "$(uname -s)" in
        Linux*)           echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        Darwin*)          echo "macos" ;;
        *)                echo "unknown" ;;
    esac
}

PLATFORM="${1:-$(detect_platform)}"
case "$PLATFORM" in
    linux|windows) ;;
    macos) fail "macOS not yet supported." ;;
    *) fail "Unknown platform: $PLATFORM. Use: install.sh [linux|windows]" ;;
esac
ok "Platform: $PLATFORM"

# ── Detect package manager (Linux) ──
PKG_MGR=""
PKG_INSTALL=""
if [[ "$PLATFORM" == "linux" ]]; then
    if command -v pacman >/dev/null 2>&1; then
        PKG_MGR="pacman"
        PKG_INSTALL="sudo pacman -S --noconfirm --needed"
    elif command -v apt-get >/dev/null 2>&1; then
        PKG_MGR="apt"
        PKG_INSTALL="sudo apt-get install -y"
    elif command -v dnf >/dev/null 2>&1; then
        PKG_MGR="dnf"
        PKG_INSTALL="sudo dnf install -y"
    elif command -v zypper >/dev/null 2>&1; then
        PKG_MGR="zypper"
        PKG_INSTALL="sudo zypper install -y"
    elif command -v apk >/dev/null 2>&1; then
        PKG_MGR="apk"
        PKG_INSTALL="sudo apk add"
    fi
fi

pkg_install() {
    if [[ -z "$PKG_INSTALL" ]]; then
        fail "No package manager found. Install '$1' manually."
    fi
    info "Installing $1 via $PKG_MGR..."
    $PKG_INSTALL "$@"
}

# ── Platform-specific variables ──
if [[ "$PLATFORM" == "linux" ]]; then
    INSTALL_DIR="$HOME/.local/bin"
    CONFIG_DIR="$HOME/.config/timeforged"
    DAEMON_NAME="timeforged"
    CLI_NAME="tf"
    TRAY_NAME="timeforged-tray"
    EXE=""
else
    APPDATA_WIN="${APPDATA:-$HOME/AppData/Roaming}"
    LOCALAPPDATA_WIN="${LOCALAPPDATA:-$HOME/AppData/Local}"
    INSTALL_DIR="$LOCALAPPDATA_WIN/TimeForged/bin"
    CONFIG_DIR="$APPDATA_WIN/timeforged"
    DAEMON_NAME="timeforged.exe"
    CLI_NAME="tf.exe"
    TRAY_NAME="timeforged-tray.exe"
    EXE=".exe"
fi

# ══════════════════════════════════════
# 1. INSTALL ALL DEPENDENCIES
# ══════════════════════════════════════
header "Installing dependencies..."

# ── curl ──
if ! command -v curl >/dev/null 2>&1; then
    if [[ "$PLATFORM" == "linux" ]]; then
        pkg_install curl
    fi
fi
ok "curl"

# ── git ──
if ! command -v git >/dev/null 2>&1; then
    if [[ "$PLATFORM" == "linux" ]]; then
        pkg_install git
    else
        fail "Git not found. Install from https://git-scm.com/download/win"
    fi
fi
ok "git $(git --version | awk '{print $3}')"

# ── Rust / Cargo ──
if ! command -v cargo >/dev/null 2>&1; then
    info "Rust not found — installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable 2>&1 | tail -3
    # Source cargo env for current session
    if [[ -f "$HOME/.cargo/env" ]]; then
        source "$HOME/.cargo/env"
    elif [[ -f "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    command -v cargo >/dev/null 2>&1 || fail "Cargo still not found after install. Restart your shell and re-run."
    ok "Rust installed $(rustc --version | awk '{print $2}')"
else
    ok "Rust $(rustc --version | awk '{print $2}')"
fi

# ── Node.js / npm ──
if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
    info "Node.js not found — installing..."
    if [[ "$PLATFORM" == "linux" ]]; then
        case "$PKG_MGR" in
            pacman) pkg_install nodejs npm ;;
            apt)
                # Use NodeSource for a recent version
                if ! command -v node >/dev/null 2>&1; then
                    info "Adding NodeSource repository..."
                    curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash - 2>&1 | tail -1
                    sudo apt-get install -y nodejs
                fi
                ;;
            dnf)    pkg_install nodejs npm ;;
            zypper) pkg_install nodejs npm ;;
            apk)    pkg_install nodejs npm ;;
            *)      fail "Cannot install Node.js — install manually from https://nodejs.org" ;;
        esac
    else
        # Windows Git Bash — try nvm-windows or direct download
        fail "Node.js not found. Install from https://nodejs.org and re-run."
    fi
    command -v node >/dev/null 2>&1 || fail "Node.js still not found after install."
    ok "Node.js installed $(node --version)"
else
    ok "Node $(node --version) / npm $(npm --version)"
fi

# ── jq ──
HAS_JQ=true
if ! command -v jq >/dev/null 2>&1; then
    if [[ "$PLATFORM" == "linux" ]]; then
        case "$PKG_MGR" in
            pacman) pkg_install jq ;;
            apt)    pkg_install jq ;;
            dnf)    pkg_install jq ;;
            zypper) pkg_install jq ;;
            apk)    pkg_install jq ;;
            *)      warn "Cannot install jq — Claude Code hooks will be skipped"; HAS_JQ=false ;;
        esac
    else
        warn "jq not found — Claude Code hooks will be skipped"
        HAS_JQ=false
    fi
    if command -v jq >/dev/null 2>&1; then
        ok "jq installed $(jq --version)"
    fi
else
    ok "jq $(jq --version)"
fi

# ── Build dependencies for tray (Linux: GTK3/libappindicator dev headers) ──
if [[ "$PLATFORM" == "linux" ]]; then
    NEED_GTK_DEV=false
    # Check if we can compile tray-icon (needs gtk3 + libappindicator headers)
    if ! pkg-config --exists gtk+-3.0 2>/dev/null; then
        NEED_GTK_DEV=true
    fi
    if ! pkg-config --exists appindicator3-0.1 2>/dev/null && ! pkg-config --exists ayatana-appindicator3-0.1 2>/dev/null; then
        NEED_GTK_DEV=true
    fi

    if [[ "$NEED_GTK_DEV" == "true" ]]; then
        info "Installing GTK3/AppIndicator dev packages for tray..."
        case "$PKG_MGR" in
            pacman) pkg_install gtk3 libappindicator-gtk3 pkg-config ;;
            apt)    pkg_install libgtk-3-dev libappindicator3-dev pkg-config ;;
            dnf)    pkg_install gtk3-devel libappindicator-gtk3-devel pkg-config ;;
            zypper) pkg_install gtk3-devel libappindicator3-1 pkg-config ;;
            *)      warn "Could not install GTK dev libs — tray may fail to build" ;;
        esac
    fi
    ok "GTK3/AppIndicator dev libs"
fi

# ══════════════════════════════════════
# 2. DETERMINE PROJECT ROOT
# ══════════════════════════════════════
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/Cargo.toml" ]]; then
    PROJECT_DIR="$SCRIPT_DIR"
else
    PROJECT_DIR="$(pwd)"
fi
[[ -f "$PROJECT_DIR/Cargo.toml" ]] || fail "Not in TimeForged project root. Run from the repo directory."
cd "$PROJECT_DIR"
ok "Project root: $PROJECT_DIR"

# ══════════════════════════════════════
# 3. BUILD WEB DASHBOARD
# ══════════════════════════════════════
header "Building web dashboard..."

WEB_DIR="$PROJECT_DIR/crates/timeforged/web"
[[ -d "$WEB_DIR" ]] || fail "Web directory not found at $WEB_DIR"

cd "$WEB_DIR"
info "Installing npm dependencies..."
npm install --silent 2>&1 | tail -1
ok "Dependencies installed"

info "Building Vue app..."
npx vite build --logLevel error 2>&1
ok "Dashboard built"
cd "$PROJECT_DIR"

# ══════════════════════════════════════
# 4. BUILD RUST BINARIES
# ══════════════════════════════════════
header "Building Rust binaries..."

TARGET_DIR="$PROJECT_DIR/target/release"

info "Compiling (this may take a few minutes on first run)..."
cargo build --release 2>&1 | grep -E "Compiling|Finished" || true
ok "Binaries built"

DAEMON_BIN="$TARGET_DIR/timeforged${EXE}"
CLI_BIN="$TARGET_DIR/tf${EXE}"
TRAY_BIN="$TARGET_DIR/timeforged-tray${EXE}"

[[ -f "$DAEMON_BIN" ]] || fail "Daemon binary not found"
[[ -f "$CLI_BIN" ]] || fail "CLI binary not found"
[[ -f "$TRAY_BIN" ]] || fail "Tray binary not found"
ok "timeforged daemon: $DAEMON_BIN"
ok "tf CLI:            $CLI_BIN"
ok "timeforged-tray:   $TRAY_BIN"

# ══════════════════════════════════════
# 5. INSTALL BINARIES
# ══════════════════════════════════════
header "Installing binaries..."

mkdir -p "$INSTALL_DIR"
cp "$DAEMON_BIN" "$INSTALL_DIR/$DAEMON_NAME"
cp "$CLI_BIN" "$INSTALL_DIR/$CLI_NAME"
cp "$TRAY_BIN" "$INSTALL_DIR/$TRAY_NAME"
ok "Installed to $INSTALL_DIR/"

if [[ "$PLATFORM" == "linux" ]]; then
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        # Auto-add to shell profile
        SHELL_RC=""
        if [[ -f "$HOME/.zshrc" ]]; then
            SHELL_RC="$HOME/.zshrc"
        elif [[ -f "$HOME/.bashrc" ]]; then
            SHELL_RC="$HOME/.bashrc"
        fi

        if [[ -n "$SHELL_RC" ]]; then
            if ! grep -q "\.local/bin" "$SHELL_RC" 2>/dev/null; then
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_RC"
                ok "Added ~/.local/bin to PATH in $(basename "$SHELL_RC")"
            fi
        fi
        export PATH="$HOME/.local/bin:$PATH"
    fi
else
    # Windows: add to user PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        info "Adding $INSTALL_DIR to user PATH..."
        powershell.exe -NoProfile -Command "
            \$p = [Environment]::GetEnvironmentVariable('PATH','User');
            if (\$p -notlike '*TimeForged*') {
                [Environment]::SetEnvironmentVariable('PATH', \$p + ';$INSTALL_DIR', 'User')
            }
        " 2>/dev/null && ok "Added to user PATH" || warn "Add $INSTALL_DIR to PATH manually"
        export PATH="$PATH:$INSTALL_DIR"
    fi
fi

# ══════════════════════════════════════
# 6. FIRST RUN — START DAEMON, CAPTURE API KEY
# ══════════════════════════════════════
header "Starting TimeForged daemon..."

if [[ "$PLATFORM" == "linux" ]]; then
    pkill -f "$INSTALL_DIR/timeforged" 2>/dev/null || true
else
    taskkill.exe /F /IM timeforged.exe 2>/dev/null || true
fi
sleep 1

DAEMON_LOG=$(mktemp)
"$INSTALL_DIR/$DAEMON_NAME" > "$DAEMON_LOG" 2>&1 &
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

CLI_CONFIG="$CONFIG_DIR/cli.toml"
if [[ -z "$API_KEY" && -f "$CLI_CONFIG" ]]; then
    EXISTING_KEY=$(grep -oP 'api_key\s*=\s*"\K[^"]+' "$CLI_CONFIG" 2>/dev/null || true)
    [[ -n "$EXISTING_KEY" ]] && API_KEY="$EXISTING_KEY"
fi

if [[ -n "$API_KEY" ]]; then
    mkdir -p "$CONFIG_DIR"
    cat > "$CLI_CONFIG" <<EOF
server_url = "http://127.0.0.1:6175"
api_key = "$API_KEY"
EOF
    ok "CLI config saved to $CLI_CONFIG"
fi

HEALTH=$(curl -sf http://127.0.0.1:6175/health 2>/dev/null || echo "")
[[ -z "$HEALTH" ]] && fail "Daemon not responding on port 6175"
ok "Health check passed"
rm -f "$DAEMON_LOG"

# ══════════════════════════════════════
# 7. AUTOSTART SETUP
# ══════════════════════════════════════
USE_SYSTEMD=false

if [[ "$PLATFORM" == "linux" ]]; then
    if command -v systemctl >/dev/null 2>&1; then
        header "Setting up systemd service..."

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
            info "Systemd failed — running manually"
            "$INSTALL_DIR/timeforged" > /dev/null 2>&1 &
            DAEMON_PID=$!
            sleep 2
        fi
    fi
else
    header "Setting up Windows autostart..."

    DAEMON_PATH_WIN=$(cygpath -w "$INSTALL_DIR/$DAEMON_NAME" 2>/dev/null || echo "$INSTALL_DIR/$DAEMON_NAME")
    schtasks.exe /Create /F /TN "TimeForged Daemon" \
        /TR "\"$DAEMON_PATH_WIN\"" \
        /SC ONLOGON /RL HIGHEST \
        /DELAY 0000:10 2>/dev/null && ok "Daemon scheduled task created" || warn "Could not create scheduled task"

    STARTUP_DIR="${APPDATA_WIN}/Microsoft/Windows/Start Menu/Programs/Startup"
    TRAY_PATH_WIN=$(cygpath -w "$INSTALL_DIR/$TRAY_NAME" 2>/dev/null || echo "$INSTALL_DIR/$TRAY_NAME")

    if command -v powershell.exe >/dev/null 2>&1; then
        powershell.exe -NoProfile -Command "
            \$ws = New-Object -ComObject WScript.Shell;
            \$sc = \$ws.CreateShortcut('$STARTUP_DIR\\TimeForged Tray.lnk');
            \$sc.TargetPath = '$TRAY_PATH_WIN';
            \$sc.Description = 'TimeForged system tray';
            \$sc.Save()
        " 2>/dev/null && ok "Tray → Startup folder" || warn "Add tray to Startup manually"
    fi

    info "Starting tray..."
    "$INSTALL_DIR/$TRAY_NAME" &>/dev/null &
    ok "Tray app started"
fi

# ══════════════════════════════════════
# 8. REMOTE SYNC
# ══════════════════════════════════════
header "Setting up remote sync..."

DEFAULT_REMOTE="https://timeforged.blysspeak.space"
TF_USERNAME=""
REMOTE_KEY=""
REMOTE_URL=""
REMOTE_OK=false

read -rp "$(echo -e "${ORANGE}▸${RESET} Remote server [$DEFAULT_REMOTE]: ")" REMOTE_URL
REMOTE_URL="${REMOTE_URL:-$DEFAULT_REMOTE}"

if curl -sf --max-time 5 "$REMOTE_URL/health" >/dev/null 2>&1; then
    ok "Remote server reachable"

    echo ""
    echo "  [1] New account  — create a new username on the server"
    echo "  [2] Link         — connect to an existing account with an API key"
    echo "  [3] Skip         — set up remote sync later"
    echo ""
    read -rp "$(echo -e "${ORANGE}▸${RESET} Choose [1/2/3]: ")" SYNC_CHOICE

    case "$SYNC_CHOICE" in
        1)
            read -rp "$(echo -e "${ORANGE}▸${RESET} Choose a username: ")" TF_USERNAME
            if [[ -n "$TF_USERNAME" ]]; then
                info "Registering as $TF_USERNAME on $REMOTE_URL..."
                REGISTER_OUTPUT=$("$INSTALL_DIR/$CLI_NAME" register "$TF_USERNAME" --remote "$REMOTE_URL" 2>&1) && REG_RC=0 || REG_RC=$?
                if [[ $REG_RC -eq 0 ]]; then
                    REMOTE_KEY=$(echo "$REGISTER_OUTPUT" | grep -oP 'tf_\w+' || true)
                    if [[ -n "$REMOTE_KEY" ]]; then
                        ok "Registered as $TF_USERNAME"
                        REMOTE_OK=true
                    else
                        warn "Registered but could not extract API key"
                    fi
                else
                    warn "Registration failed: $REGISTER_OUTPUT"
                fi
            fi
            ;;
        2)
            read -rp "$(echo -e "${ORANGE}▸${RESET} Paste your remote API key (tf_...): ")" REMOTE_KEY
            if [[ "$REMOTE_KEY" =~ ^tf_[a-zA-Z0-9]+$ ]]; then
                info "Linking to existing account..."
                LINK_OUTPUT=$("$INSTALL_DIR/$CLI_NAME" link "$REMOTE_KEY" --remote "$REMOTE_URL" 2>&1) && LINK_RC=0 || LINK_RC=$?
                if [[ $LINK_RC -eq 0 ]]; then
                    ok "Linked to existing account"
                    REMOTE_OK=true
                else
                    warn "Link failed: $LINK_OUTPUT"
                fi
            else
                warn "Invalid key format (expected tf_...)"
            fi
            ;;
        *)
            info "Skipping remote setup"
            echo -e "  ${DIM}Run later: $CLI_NAME register <username> --remote $REMOTE_URL${RESET}"
            echo -e "  ${DIM}Or link:   $CLI_NAME link <api-key> --remote $REMOTE_URL${RESET}"
            ;;
    esac
else
    warn "Remote unreachable — skipping. Run later:"
    echo -e "  ${DIM}$CLI_NAME register <username> --remote $REMOTE_URL${RESET}"
    echo -e "  ${DIM}Or link:   $CLI_NAME link <api-key> --remote $REMOTE_URL${RESET}"
fi

if [[ "$REMOTE_OK" == "true" && -n "$API_KEY" ]]; then
    if [[ -z "$REMOTE_KEY" ]]; then
        # Link command saves config itself, just add remote_url if missing
        if ! grep -q "remote_url" "$CLI_CONFIG" 2>/dev/null; then
            echo "remote_url = \"$REMOTE_URL\"" >> "$CLI_CONFIG"
        fi
    else
        cat > "$CLI_CONFIG" <<EOF
server_url = "http://127.0.0.1:6175"
api_key = "$API_KEY"
remote_url = "$REMOTE_URL"
remote_key = "$REMOTE_KEY"
EOF
    fi
    ok "CLI config updated with remote"

    "$INSTALL_DIR/$CLI_NAME" profile --public 2>&1 || true
    "$INSTALL_DIR/$CLI_NAME" sync 2>&1 || true
fi

# ── Auto-sync timer ──
if [[ "$REMOTE_OK" == "true" ]]; then
    if [[ "$PLATFORM" == "linux" && "$USE_SYSTEMD" == "true" ]]; then
        SYSTEMD_DIR="$HOME/.config/systemd/user"
        cat > "$SYSTEMD_DIR/tf-sync.service" <<EOF
[Unit]
Description=TimeForged sync
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=$INSTALL_DIR/tf sync
Environment=HOME=$HOME
EOF
        cat > "$SYSTEMD_DIR/tf-sync.timer" <<EOF
[Unit]
Description=TimeForged auto-sync

[Timer]
OnCalendar=*:0/15
Persistent=true
RandomizedDelaySec=60

[Install]
WantedBy=timers.target
EOF
        systemctl --user daemon-reload
        systemctl --user enable --now tf-sync.timer
        ok "Auto-sync timer (15 min)"

    elif [[ "$PLATFORM" == "windows" ]]; then
        CLI_PATH_WIN=$(cygpath -w "$INSTALL_DIR/$CLI_NAME" 2>/dev/null || echo "$INSTALL_DIR/$CLI_NAME")
        schtasks.exe /Create /F /TN "TimeForged Sync" \
            /TR "\"$CLI_PATH_WIN\" sync" \
            /SC MINUTE /MO 15 2>/dev/null && ok "Auto-sync task (15 min)" || warn "Could not create sync task"
    fi
fi

# ══════════════════════════════════════
# 9. CLAUDE CODE HOOKS
# ══════════════════════════════════════
CLAUDE_DIR=""
[[ "$PLATFORM" == "linux" ]] && CLAUDE_DIR="$HOME/.claude" || CLAUDE_DIR="${USERPROFILE:-$HOME}/.claude"

CLAUDE_HOOKS_INSTALLED=false
if [[ -d "$CLAUDE_DIR" && "$HAS_JQ" == "true" ]]; then
    header "Configuring Claude Code hooks..."

    mkdir -p "$CLAUDE_DIR/hooks"
    cp "$PROJECT_DIR/contrib/claude-code/timeforged-heartbeat.sh" "$CLAUDE_DIR/hooks/timeforged-heartbeat.sh"
    chmod +x "$CLAUDE_DIR/hooks/timeforged-heartbeat.sh" 2>/dev/null || true
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
fi

# ══════════════════════════════════════
# 10. PLATFORM INTEGRATIONS
# ══════════════════════════════════════
if [[ "$PLATFORM" == "linux" ]]; then
    if command -v waybar >/dev/null 2>&1; then
        header "Installing Waybar module..."
        [[ -f "$PROJECT_DIR/contrib/waybar/install.sh" ]] && bash "$PROJECT_DIR/contrib/waybar/install.sh"
    fi
fi

# ══════════════════════════════════════
# SUMMARY
# ══════════════════════════════════════
echo ""
echo -e "${BOLD}${ORANGE}  ╔══════════════════════════════════════╗${RESET}"
echo -e "${BOLD}${ORANGE}  ║       Installation Complete!         ║${RESET}"
echo -e "${BOLD}${ORANGE}  ╚══════════════════════════════════════╝${RESET}"
echo ""

echo -e "  ${BOLD}Platform:${RESET}    ${GREEN}${PLATFORM}${RESET}"
echo -e "  ${BOLD}Dashboard:${RESET}   ${GREEN}http://127.0.0.1:6175${RESET}"
[[ -n "$API_KEY" ]] && echo -e "  ${BOLD}API Key:${RESET}     ${ORANGE}${API_KEY}${RESET}"

if [[ "$REMOTE_OK" == "true" ]]; then
    echo -e "  ${BOLD}Card URL:${RESET}    ${GREEN}${REMOTE_URL%/}/github/timeforged/${TF_USERNAME}.svg${RESET}"
    echo -e "  ${BOLD}Auto-sync:${RESET}   every 15 min"
fi

[[ "$CLAUDE_HOOKS_INSTALLED" == "true" ]] && echo -e "  ${BOLD}Claude Code:${RESET} hooks installed"

echo ""
echo -e "  ${BOLD}Config:${RESET}      ${DIM}${CONFIG_DIR}/cli.toml${RESET}"
echo -e "  ${BOLD}Binaries:${RESET}    ${DIM}${INSTALL_DIR}/${RESET}"
[[ "$PLATFORM" == "windows" ]] && echo -e "  ${BOLD}Tray:${RESET}        ${GREEN}running in system tray${RESET}"
echo ""

if [[ "$REMOTE_OK" == "true" ]]; then
    echo -e "  ${BOLD}GitHub README:${RESET}"
    echo -e "    ${GREEN}<img src=\"${REMOTE_URL%/}/github/timeforged/${TF_USERNAME}.svg\" />${RESET}"
    echo ""
fi

echo -e "  ${BOLD}Quick start:${RESET}"
if [[ "$PLATFORM" == "linux" ]]; then
    echo -e "    ${GREEN}tf init ~/projects${RESET}  — start tracking"
else
    echo -e "    ${GREEN}tf init C:\\projects${RESET} — start tracking"
fi
echo -e "    ${DIM}tf today${RESET}            — today's summary"
echo -e "    ${DIM}tf status${RESET}           — daemon status"
echo -e "    ${DIM}tf sync${RESET}             — manual sync"
echo ""

if [[ "$PLATFORM" == "linux" ]]; then
    if [[ "$USE_SYSTEMD" == "true" ]]; then
        echo -e "  ${BOLD}Systemd:${RESET}"
        echo -e "    ${DIM}systemctl --user status timeforged${RESET}"
        echo -e "    ${DIM}journalctl --user -u timeforged -f${RESET}"
    fi
else
    echo -e "  ${BOLD}Windows:${RESET}"
    echo -e "    ${DIM}Daemon → Task Scheduler (at logon)${RESET}"
    echo -e "    ${DIM}Tray   → Startup folder${RESET}"
    echo -e "    ${DIM}Right-click tray → Open Dashboard / Quit${RESET}"
fi
echo ""
