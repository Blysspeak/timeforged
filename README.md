<p align="center">
  <img src="timeforged-logo.png" width="280" alt="TimeForged" />
</p>

<p align="center">
  <a href="https://github.com/Blysspeak/timeforged/blob/main/LICENSE"><img src="https://img.shields.io/github/license/Blysspeak/timeforged" alt="license" /></a>
</p>

<p align="center">
  Self-hosted time tracking for developers. Daemon + CLI + Web Dashboard + MCP, written in Rust.
</p>

---

## Quick Start

```bash
git clone https://github.com/Blysspeak/timeforged.git
cd timeforged
bash install.sh
```

The installer will:
- Build the web dashboard (Vue 3 + Tailwind)
- Compile Rust binaries (`timeforged` daemon + `tf` CLI)
- Install to `~/.local/bin/`
- Set up a systemd user service
- Install the Waybar module (if Waybar is detected)
- Start the daemon and display your API key

Then initialize tracking:

```bash
tf init ~/projects    # watch a directory tree
```

Open **http://127.0.0.1:6175** in your browser — the dashboard works immediately, no login required on localhost.

### Manual build

```bash
# Build dashboard
cd crates/timeforged/web && npm install && npx vite build && cd ../../..

# Build binaries
cargo build --release

# Run daemon
./target/release/timeforged
```

On first run, an admin user and API key are created automatically:

```
==============================================
  TimeForged — first run setup
  Created admin user with API key:
  tf_abc123...
  Save this key! It won't be shown again.
==============================================
```

## Web Dashboard

The daemon serves a built-in web dashboard at `http://127.0.0.1:6175`:

- **Activity chart** — time per day (last 7 days)
- **Projects & Languages** — breakdown with progress bars and language icons
- **Sessions** — recent coding sessions with duration
- **Stats** — total time, top project, session count, events

Localhost requests are auto-authenticated — no API key needed to view the dashboard.

## File Watcher

TimeForged automatically tracks your coding activity via filesystem events — no editor plugins needed.

```bash
tf init ~/projects          # start watching (recursive, inotify-based)
tf list                     # show watched directories
tf unwatch ~/projects       # stop watching
```

The daemon watches registered directories recursively and creates heartbeat events on file changes. Features:
- **Project detection** — first-level subdirectory of the watched root becomes the project name
- **Language detection** — inferred from file extension and filename patterns
- **Git branch** — cached per project (60s TTL)
- **Debounce** — 30s per file to avoid event spam
- **Ignored paths** — `.git`, `node_modules`, `target`, `__pycache__`, lock files, binaries
- **Window tracker** (optional) — polls `hyprctl` / `xdotool` every 15s for active editor file

Watched directories persist in `~/.config/timeforged/watched.toml`.

## Waybar Module

Show today's coding time in your Waybar panel:

```bash
bash contrib/waybar/install.sh
```

This installs a custom module that queries the TimeForged API every 60s, displays time as `󱑂 1:25`, and opens the dashboard on click. The tooltip shows per-project breakdown.

The installer is also run automatically by `install.sh` when Waybar is detected.

## Architecture

```
crates/
  timeforged-core/   # Shared types, models, config
  timeforged/        # Daemon — Axum REST API + SQLite + embedded SPA
    web/             # Vue 3 + Tailwind CSS dashboard
  tf/                # CLI client
contrib/
  waybar/            # Waybar module + installer
```

```
File Watcher (inotify) ──┐
Window Tracker (optional) ┼──→ Events ──→ Storage (SQLite)
HTTP API (POST /events) ──┘
                                            ↓
Browser → Embedded SPA (rust-embed) ──→ Reports API ──→ Storage
Waybar module ─────────────────────────→ Reports API
```

## CLI

```bash
tf init ~/projects              # watch a directory tree
tf list                         # show watched directories
tf unwatch ~/projects           # stop watching

tf status                       # daemon status
tf today                        # today's summary
tf report --range week          # weekly report
tf report --range month --project myapp

tf send /path/to/file.rs --project myapp --language Rust  # manual heartbeat
```

API key is configured once in `~/.config/timeforged/cli.toml` or via `TF_API_KEY`.

## Docker

```bash
docker compose up -d
```

Or build manually:

```bash
docker build -t timeforged .
docker run -d -p 6175:6175 -v timeforged-data:/data timeforged
```

## REST API

All authenticated endpoints require the `X-Api-Key` header (or localhost access).

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check (no auth) |
| GET | `/api/v1/status` | Daemon status (no auth) |
| POST | `/api/v1/events` | Create event |
| POST | `/api/v1/events/batch` | Batch create (up to 100) |
| GET | `/api/v1/reports/summary` | Time summary by project/language/day |
| GET | `/api/v1/reports/sessions` | Coding sessions |
| GET | `/api/v1/reports/activity` | Hourly activity |
| GET | `/api/v1/me` | Current user |
| POST | `/api/v1/api-keys` | Create API key |
| GET | `/api/v1/api-keys` | List API keys |
| DELETE | `/api/v1/api-keys/{id}` | Delete API key |
| POST | `/api/v1/watch` | Add watched directory |
| DELETE | `/api/v1/watch` | Remove watched directory |
| GET | `/api/v1/watched` | List watched directories |

### Query parameters

`from`, `to` (ISO 8601), `project`, `language`

### Event body

```json
{
  "timestamp": "2026-02-27T10:00:00Z",
  "event_type": "file",
  "entity": "/home/user/project/src/main.rs",
  "project": "app",
  "language": "Rust",
  "branch": "main",
  "activity": "coding"
}
```

`event_type`: `file` | `terminal` | `browser` | `meeting` | `custom`
`activity`: `coding` | `browsing` | `debugging` | `building` | `communicating` | `designing` | `other`

Project and language are auto-inferred from file path when omitted.

## Configuration

### Daemon — `~/.config/timeforged/config.toml`

```toml
host = "127.0.0.1"
port = 6175
database_url = "sqlite:~/.local/share/timeforged/timeforged.db?mode=rwc"
idle_timeout = 300
log_level = "info"
```

### CLI — `~/.config/timeforged/cli.toml`

```toml
server_url = "http://127.0.0.1:6175"
api_key = "tf_..."
```

### Watched directories — `~/.config/timeforged/watched.toml`

```toml
[[directories]]
path = "/home/user/projects"
```

Managed via `tf init` / `tf unwatch`.

### Environment variables

All settings can be overridden with `TF_` prefix:

`TF_HOST`, `TF_PORT`, `TF_DATABASE_URL`, `TF_IDLE_TIMEOUT`, `TF_LOG_LEVEL`, `TF_SERVER_URL`, `TF_API_KEY`

## License

[MIT](LICENSE)
