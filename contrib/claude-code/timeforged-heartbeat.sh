#!/usr/bin/env bash
# TimeForged heartbeat hook for Claude Code
# Sends events to the TimeForged daemon on every interaction.
# Fires on: UserPromptSubmit, PostToolUse, Stop
# Runs curl in the background so it never blocks Claude Code.

# Config path: Windows (%APPDATA%) or Linux/macOS (~/.config)
if [[ -n "$APPDATA" ]]; then
  TF_CONFIG="$APPDATA/timeforged/cli.toml"
elif [[ "$(uname)" == "Darwin" ]]; then
  TF_CONFIG="${HOME}/Library/Application Support/timeforged/cli.toml"
else
  TF_CONFIG="${HOME}/.config/timeforged/cli.toml"
fi
TF_SERVER_URL="${TF_SERVER_URL:-http://127.0.0.1:6175}"
TF_API_KEY="${TF_API_KEY:-$(grep -oP 'api_key\s*=\s*"\K[^"]+' "$TF_CONFIG" 2>/dev/null || grep -o 'api_key *= *"[^"]*"' "$TF_CONFIG" 2>/dev/null | sed 's/.*"\(.*\)"/\1/')}"

# No key = no tracking
[[ -z "$TF_API_KEY" ]] && exit 0

INPUT="$(cat)"

# Debounce: skip if last heartbeat was <30s ago
STAMP="${TMPDIR:-${TMP:-/tmp}}/.tf-heartbeat-stamp"
NOW=$(date +%s)
if [[ -f "$STAMP" ]]; then
  LAST=$(cat "$STAMP")
  (( NOW - LAST < 30 )) && exit 0
fi
echo "$NOW" > "$STAMP"

# Extract context from hook input
CWD=$(echo "$INPUT" | jq -r '.cwd // empty' 2>/dev/null)
EVENT_NAME=$(echo "$INPUT" | jq -r '.hook_event_name // empty' 2>/dev/null)

# Try to get file path from tool_input (file operations)
FILE_PATH=$(echo "$INPUT" | jq -re '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null) || true

# Determine entity and event_type
if [[ -n "$FILE_PATH" ]]; then
  ENTITY="$FILE_PATH"
  EVENT_TYPE="file"
  # Language detection from extension
  EXT="${FILE_PATH##*.}"
  case "$EXT" in
    rs)       LANG="Rust" ;;
    ts|tsx)   LANG="TypeScript" ;;
    js|jsx)   LANG="JavaScript" ;;
    py)       LANG="Python" ;;
    sh)       LANG="Shell" ;;
    vue)      LANG="Vue" ;;
    toml)     LANG="TOML" ;;
    json)     LANG="JSON" ;;
    md)       LANG="Markdown" ;;
    sql)      LANG="SQL" ;;
    css|scss) LANG="CSS" ;;
    html)     LANG="HTML" ;;
    go)       LANG="Go" ;;
    java)     LANG="Java" ;;
    yml|yaml) LANG="YAML" ;;
    *)        LANG="" ;;
  esac
else
  ENTITY="${CWD:-unknown}"
  EVENT_TYPE="custom"
  LANG=""
fi

# Detect project name from git repo root
PROJECT=""
DIR="${CWD:-$(dirname "$FILE_PATH" 2>/dev/null)}"
if [[ -n "$DIR" && -d "$DIR" ]]; then
  PROJECT=$(cd "$DIR" 2>/dev/null && basename "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null) || true
fi

# Detect branch
BRANCH=""
if [[ -n "$DIR" && -d "$DIR" ]]; then
  BRANCH=$(cd "$DIR" 2>/dev/null && git rev-parse --abbrev-ref HEAD 2>/dev/null) || true
fi

TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)

PAYLOAD=$(jq -n \
  --arg ts "$TIMESTAMP" --arg entity "$ENTITY" --arg et "$EVENT_TYPE" \
  --arg project "$PROJECT" --arg language "$LANG" --arg branch "$BRANCH" \
  --arg source "$EVENT_NAME" \
  '{
    timestamp: $ts,
    event_type: $et,
    entity: $entity,
    activity: "coding",
    project: (if $project == "" then null else $project end),
    language: (if $language == "" then null else $language end),
    branch: (if $branch == "" then null else $branch end),
    metadata: { source: "claude-code", hook: $source }
  }')

# Send in background, ignore failures
curl -s -o /dev/null --max-time 5 \
  -X POST -H "Content-Type: application/json" -H "X-Api-Key: $TF_API_KEY" \
  -d "$PAYLOAD" "${TF_SERVER_URL}/api/v1/events" &

exit 0
