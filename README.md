# TimeForged

Self-hosted time tracking daemon and CLI, written in Rust.

## Architecture

```
crates/
  timeforged-core/   # Shared types, models, config, error handling
  timeforged/        # Daemon — Axum REST API + SQLite storage
  tf/                # CLI client
```

**Daemon layers:**
```
HTTP → Auth Middleware (X-Api-Key) → Handler → Service → Storage (SQLite)
```

## Quick Start

### Build

```bash
cargo build --release
```

### Run the daemon

```bash
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

Default bind address: `127.0.0.1:6175`.

### CLI usage

```bash
# Check daemon status
tf --key tf_abc123... status

# Send a heartbeat event
tf --key tf_abc123... send /path/to/file.rs --project myapp --language Rust

# Today's summary
tf --key tf_abc123... today

# Weekly report
tf --key tf_abc123... report --range week

# Report with project filter
tf --key tf_abc123... report --range month --project myapp
```

## REST API

All authenticated endpoints require the `X-Api-Key` header.

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check (no auth) |
| GET | `/api/v1/status` | Daemon status (no auth) |
| POST | `/api/v1/events` | Create a single event |
| POST | `/api/v1/events/batch` | Create up to 100 events |
| GET | `/api/v1/reports/summary` | Summary: total time, by project/language/day |
| GET | `/api/v1/reports/sessions` | List of sessions (idle gap = 5 min) |
| GET | `/api/v1/reports/activity` | Hourly activity breakdown |
| GET | `/api/v1/me` | Current user info |
| POST | `/api/v1/api-keys` | Create a new API key |
| GET | `/api/v1/api-keys` | List API keys |
| DELETE | `/api/v1/api-keys/{id}` | Delete an API key |

### Report query parameters

- `from` — start date (ISO 8601, e.g. `2026-01-01T00:00:00Z`)
- `to` — end date
- `project` — filter by project name
- `language` — filter by language

### Event body

```json
{
  "timestamp": "2026-02-27T10:00:00Z",
  "event_type": "file",
  "entity": "/home/user/projects/app/src/main.rs",
  "project": "app",
  "language": "Rust",
  "branch": "main",
  "activity": "coding",
  "machine": "laptop",
  "metadata": {}
}
```

`event_type`: `file`, `terminal`, `browser`, `meeting`, `custom`
`activity`: `coding`, `browsing`, `debugging`, `building`, `communicating`, `designing`, `other`

If `project` or `language` are omitted, the daemon infers them from the file path.

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

All settings can be overridden with `TF_` prefixed env vars:

`TF_HOST`, `TF_PORT`, `TF_DATABASE_URL`, `TF_IDLE_TIMEOUT`, `TF_LOG_LEVEL`, `TF_SERVER_URL`, `TF_API_KEY`

## License

MIT
