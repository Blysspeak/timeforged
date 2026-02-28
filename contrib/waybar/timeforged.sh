#!/usr/bin/env bash
# TimeForged waybar module — shows today's coding time

CONFIG="$HOME/.config/timeforged/cli.toml"
API_KEY=""
SERVER="http://127.0.0.1:6175"

if [[ -f "$CONFIG" ]]; then
    API_KEY=$(grep -oP 'api_key\s*=\s*"\K[^"]+' "$CONFIG" 2>/dev/null || true)
    srv=$(grep -oP 'server_url\s*=\s*"\K[^"]+' "$CONFIG" 2>/dev/null || true)
    [[ -n "$srv" ]] && SERVER="$srv"
fi

TODAY=$(date -u +%Y-%m-%dT00:00:00Z)
TOMORROW=$(date -u -d "+1 day" +%Y-%m-%dT00:00:00Z)

RESPONSE=$(curl -sf --max-time 3 \
    -H "X-Api-Key: ${API_KEY}" \
    "${SERVER}/api/v1/reports/summary?from=${TODAY}&to=${TOMORROW}" 2>/dev/null)

if [[ -z "$RESPONSE" ]]; then
    printf '{"text":"󱑂  offline","tooltip":"Daemon not responding","class":"offline"}\n'
    exit 0
fi

TOTAL=$(echo "$RESPONSE" | jq -r '.total_seconds // 0' 2>/dev/null)
if [[ -z "$TOTAL" || "$TOTAL" == "null" ]]; then
    printf '{"text":"󱑂  offline","tooltip":"Failed to parse","class":"offline"}\n'
    exit 0
fi

HOURS=$(echo "$TOTAL" | awk '{printf "%d", $1 / 3600}')
MINS=$(echo "$TOTAL" | awk '{printf "%d", ($1 % 3600) / 60}')

# Tooltip: per-project breakdown (skip 0m, trim paths to basename)
TOOLTIP=$(echo "$RESPONSE" | jq -r '
    .projects // [] | sort_by(-.total_seconds) |
    map(select(.total_seconds >= 60)) |
    map(
        (.total_seconds / 3600 | floor | tostring) + ":" +
        ((.total_seconds % 3600 / 60) | floor | tostring | if length == 1 then "0" + . else . end) + "  " +
        (.name | split("/") | last)
    ) | join("\n")
' 2>/dev/null)

[[ -z "$TOOLTIP" ]] && TOOLTIP="no projects today"

CLASS="active"
[[ "$HOURS" -eq 0 && "$MINS" -eq 0 ]] && CLASS="idle"

TOOLTIP_JSON=$(printf '%s' "$TOOLTIP" | jq -Rsc '.')
printf '{"text":"󱑂  %d:%02d","tooltip":%s,"class":"%s"}\n' \
    "$HOURS" "$MINS" "$TOOLTIP_JSON" "$CLASS"
