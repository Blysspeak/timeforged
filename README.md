<p align="center">
  <img src="timeforged-logo.png" width="280" alt="TimeForged" />
</p>

<p align="center">
  <a href="https://github.com/Blysspeak/timeforged/blob/main/LICENSE"><img src="https://img.shields.io/github/license/Blysspeak/timeforged" alt="license" /></a>
  <a href="https://www.npmjs.com/package/timeforged-mcp"><img src="https://img.shields.io/npm/v/timeforged-mcp?label=MCP" alt="MCP" /></a>
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
- Start the daemon and display your API key
- Show the dashboard URL

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

## Architecture

```
crates/
  timeforged-core/   # Shared types, models, config
  timeforged/        # Daemon — Axum REST API + SQLite + embedded SPA
    web/             # Vue 3 + Tailwind CSS dashboard
  tf/                # CLI client
```

```
Browser → Embedded SPA (rust-embed)
HTTP API → CORS → Auth Middleware (X-Api-Key / localhost auto-auth) → Handler → Service → Storage (SQLite)
```

## CLI

```bash
# Check daemon status
tf status

# Today's summary
tf today

# Weekly report
tf report --range week

# Report filtered by project
tf report --range month --project myapp

# Send a heartbeat
tf send /path/to/file.rs --project myapp --language Rust
```

API key is configured once in `~/.config/timeforged/cli.toml` or via `TF_API_KEY`.

## MCP Integration

Connect your AI assistant to TimeForged with the [MCP server](https://github.com/Blysspeak/timeforged-mcp):

```bash
npx timeforged-mcp
```

### Claude Code

```bash
claude mcp add timeforged \
  --transport stdio \
  --env TF_API_KEY=your-api-key \
  -- npx -y timeforged-mcp
```

### Claude Desktop / Cursor / VS Code

```json
{
  "mcpServers": {
    "timeforged": {
      "command": "npx",
      "args": ["-y", "timeforged-mcp"],
      "env": {
        "TF_API_KEY": "your-api-key"
      }
    }
  }
}
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

### Environment variables

All settings can be overridden with `TF_` prefix:

`TF_HOST`, `TF_PORT`, `TF_DATABASE_URL`, `TF_IDLE_TIMEOUT`, `TF_LOG_LEVEL`, `TF_SERVER_URL`, `TF_API_KEY`

## License

[MIT](LICENSE)
