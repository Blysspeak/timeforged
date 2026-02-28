#!/usr/bin/env bash
set -euo pipefail

# TimeForged Waybar Module Installer
# Adds a coding time widget with logo to your Waybar panel

BOLD='\033[1m'
DIM='\033[2m'
ORANGE='\033[38;5;214m'
GREEN='\033[32m'
RED='\033[31m'
RESET='\033[0m'

info()  { echo -e "${ORANGE}▸${RESET} $1"; }
ok()    { echo -e "${GREEN}✓${RESET} $1"; }
fail()  { echo -e "${RED}✗${RESET} $1"; exit 1; }

echo ""
echo -e "${BOLD}${ORANGE}  TimeForged — Waybar Module${RESET}"
echo ""

# ── Check dependencies ──
command -v jq >/dev/null 2>&1 || fail "jq is required. Install: sudo pacman -S jq"
command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v waybar >/dev/null 2>&1 || fail "waybar not found"

# ── Find sources ──
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_SCRIPT="$SCRIPT_DIR/timeforged.sh"
[[ -f "$SRC_SCRIPT" ]] || fail "timeforged.sh not found in $SCRIPT_DIR"

# Find project root for icon
PROJECT_ROOT="$SCRIPT_DIR/../.."
ICON_SRC="$PROJECT_ROOT/crates/timeforged/web/public/favicon-48.png"

# ── Install script + icon ──
WAYBAR_DIR="$HOME/.config/waybar"
MODULES_DIR="$WAYBAR_DIR/modules"
mkdir -p "$MODULES_DIR"

cp "$SRC_SCRIPT" "$MODULES_DIR/timeforged.sh"
chmod +x "$MODULES_DIR/timeforged.sh"
ok "Script installed to $MODULES_DIR/timeforged.sh"

if [[ -f "$ICON_SRC" ]]; then
    cp "$ICON_SRC" "$WAYBAR_DIR/timeforged-icon.png"
    ok "Icon installed to $WAYBAR_DIR/timeforged-icon.png"
else
    info "Icon not found at $ICON_SRC — module will work without logo"
fi

# ── Patch waybar config ──
WAYBAR_CONFIG="$WAYBAR_DIR/config"
if [[ ! -f "$WAYBAR_CONFIG" ]]; then
    fail "Waybar config not found at $WAYBAR_CONFIG"
fi

if grep -q "custom/timeforged" "$WAYBAR_CONFIG"; then
    info "Module already in waybar config, skipping"
else
    # Add to modules-right before "tray"
    if grep -q '"tray"' "$WAYBAR_CONFIG"; then
        sed -i 's/"tray"/"image#timeforged",\n        "custom\/timeforged",\n        "tray"/' "$WAYBAR_CONFIG"
        ok "Added to modules-right (before tray)"
    else
        info "Could not find tray module — add \"image#timeforged\" and \"custom/timeforged\" to modules-right manually"
    fi

    # Add module definitions before closing }
    DEFINITION=",\n\n    \"image#timeforged\": {\n        \"path\": \"${HOME}/.config/waybar/timeforged-icon.png\",\n        \"size\": 26,\n        \"tooltip\": false,\n        \"on-click\": \"xdg-open http://127.0.0.1:6175\"\n    },\n\n    \"custom/timeforged\": {\n        \"format\": \"{}\",\n        \"return-type\": \"json\",\n        \"interval\": 60,\n        \"exec\": \"~/.config/waybar/modules/timeforged.sh\",\n        \"on-click\": \"xdg-open http://127.0.0.1:6175\"\n    }"

    sed -i '$ s/}/'"$(echo -e "$DEFINITION")"'\n}/' "$WAYBAR_CONFIG"
    ok "Module definitions added to config"
fi

# ── Patch waybar style ──
WAYBAR_STYLE="$WAYBAR_DIR/style.css"
if [[ ! -f "$WAYBAR_STYLE" ]]; then
    info "No style.css found, skipping style injection"
else
    if grep -q "custom-timeforged" "$WAYBAR_STYLE"; then
        info "Styles already present, skipping"
    else
        cat >> "$WAYBAR_STYLE" <<'CSS'

#image-timeforged {
    background-color: #111827;
    border-radius: 14px 0 0 14px;
    padding: 4px 2px 4px 8px;
    margin-right: 0;
}

#custom-timeforged {
    background-color: #111827;
    border-radius: 0 14px 14px 0;
    padding: 3px 10px 3px 4px;
    margin-left: 0;
    color: #a6e3a1;
    font-size: 12px;
}

#custom-timeforged.offline {
    color: #6c7086;
}

#custom-timeforged.idle {
    color: #585b70;
}
CSS
        ok "Styles added to style.css"
    fi
fi

# ── Reload waybar ──
if pgrep -x waybar >/dev/null 2>&1; then
    pkill -SIGUSR2 waybar
    ok "Waybar reloaded"
else
    info "Waybar not running — start it to see the module"
fi

# ── Test ──
echo ""
info "Testing script..."
OUTPUT=$(bash "$MODULES_DIR/timeforged.sh")
echo -e "  ${DIM}${OUTPUT}${RESET}"
echo ""
ok "Done! TimeForged widget with logo should appear in your Waybar panel."
echo ""
