# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

This is a Cargo workspace with a separate Node-based frontend and MCP server.

```bash
# Full build (dashboard + all Rust crates)
cd crates/timeforged/web && npm install && npx vite build && cd ../../..
cargo build --release

# Individual crate build
cargo build -p timeforged        # daemon
cargo build -p tf                # CLI
cargo build -p timeforged-tray   # tray app
cargo build -p timeforged-core   # shared library

# Run daemon (from source)
cargo run -p timeforged
# Or from release:
./target/release/timeforged

# Run CLI
./target/release/tf <command>

# MCP server (separate Node package, NOT in workspace)
cd crates/timeforged-mcp && npm install && npm run build
```

The daemon serves the Vue dashboard via `rust-embed` from `crates/timeforged/web/dist/` — **the web bundle must be built before `cargo build`** or the embedded SPA will be empty/stale.

## Testing

```bash
cargo test                          # all workspace tests (≈50 unit tests)
cargo test -p timeforged-core       # single crate
cargo test --lib util::tests::      # specific module filter
cargo test -- --nocapture           # show println! output
```

Tests live inside `#[cfg(test)] mod tests` blocks next to the code (no `tests/` directory). Notable test modules: `timeforged-core/{util,config,models/event}`, `timeforged/watcher/{debounce,window_tracker}`, `timeforged-tray/poller`.

## Web Dashboard Dev Loop

```bash
cd crates/timeforged/web
npm run dev        # vite dev server (proxies to running daemon on :6175)
npm run build      # vue-tsc type check + vite build → dist/
```

## Installers

- `install.sh` — cross-platform (Linux + Windows/Git Bash); installs Rust/Node/jq, builds, registers systemd unit + timer.
- `install.ps1` — Windows PowerShell; uses winget/scoop/choco, configures Task Scheduler + Startup.

These orchestrate the full user-facing flow (build → install to `~/.local/bin` → register service → start daemon → print API key → register on remote `timeforged.nexalix.io`).

## Architecture

### Crate layout

```
crates/
  timeforged-core/   # Shared models, config, errors, util (no I/O beyond config files)
  timeforged/        # Daemon: Axum REST API + SQLite + embedded SPA + file watcher + auto-sync
    web/             # Vue 3 + Tailwind 4 + vue-chartjs — embedded into daemon via rust-embed
  tf/                # CLI client (clap subcommands, talks to daemon over HTTP)
  timeforged-tray/   # Cross-platform system tray icon (tray-icon crate); polls daemon API
  timeforged-mcp/    # Node/TypeScript MCP server — NOT a Cargo member, separate npm package
```

### Data flow

All event sources (`file watcher` via `notify` inotify, `window tracker` polling `hyprctl`/`xdotool`/Win32, `Claude Code` hooks in `contrib/claude-code`, external `POST /api/v1/events`) converge on SQLite storage. The dashboard and Waybar module read back through the Reports API. There is no direct DB access from clients — everything goes through the daemon.

### Daemon internals (`crates/timeforged/src/`)

- `main.rs` — spawns: file_watcher task, window_tracker task, auto-sync task (periodic remote push), Axum server. Creates admin user on first run (prints API key once, never shown again).
- `app.rs` — router wiring. Two route groups: `authed` (requires `X-Api-Key` via `auth::auth_middleware`) and `public` (`/health`, `/api/v1/status`, `/api/v1/register`, public SVG cards). Security headers (`X-Frame-Options: DENY`, `nosniff`, `Referrer-Policy`) and CORS applied at the outer layer.
- `auth.rs` — API-key middleware. Keys are hashed (SHA-256) in DB; lookup via `user_service::authenticate`.
- `rate_limit.rs` — per-IP limiters: 120 events/min, separate limiter on `/register`.
- `web.rs` — SPA fallback handler. **Critical security rule:** auto-injects the CLI's API key into dashboard HTML *only when daemon binds to `127.0.0.1`/`localhost`* — prevents key leakage on public `0.0.0.0` deployments (see v0.4.1 security fix).
- `sync.rs` — background task that pushes local events to `remote_url` on `sync_interval` (default 300s).
- `handlers/` — thin HTTP layer; `service/` — business logic; `storage/sqlite.rs` — all SQL queries; `storage/migrations/*.sql` — `include_str!`'d and run at startup (additive, ignore-errors style for idempotency).
- `watcher/` — `file_watcher` uses `notify` recursive inotify with 30s per-file debounce and ignore globs (`.git`, `node_modules`, `target`, etc.). Project name = first-level subdirectory of the watched root. Language inferred from extension. Git branch cached 60s. `window_tracker` polls every 15s and only emits when the foreground window's path is inside a watched directory.

### Cross-platform path handling (v0.5.0)

Always use:
- `dirs::config_dir()` → `~/.config` on Linux, `%APPDATA%` on Windows
- `dirs::data_dir()` → `~/.local/share` on Linux, `%LOCALAPPDATA%` on Windows
- `gethostname::gethostname()` instead of reading `/etc/hostname`

Helpers `config_dir()` / data-dir resolution live in `timeforged-core/src/config.rs`. **Don't hardcode `$HOME/.config` or Unix-only paths.**

Windows-only deps (`windows-sys` for `GetForegroundWindow`) are gated in `crates/timeforged/Cargo.toml` with `[target.'cfg(windows)'.dependencies]`.

### Config surface

Three TOML files under the config dir, each with env-var overrides (`TF_*` prefix):
- `config.toml` — daemon (`host`, `port`, `database_url`, `idle_timeout`, `log_level`, `sync_interval`)
- `cli.toml` — CLI (`server_url`, `api_key`, `remote_url`, `remote_key`)
- `watched.toml` — `[[directories]]` list managed via `tf init` / `tf unwatch`

### Remote sync model

Two-daemon topology: user's local daemon (localhost) syncs to a central public daemon at `timeforged.nexalix.io` (same codebase, different config). `tf register` creates an account + remote key; `tf sync` pushes events in batches of 100 (server caps reads at 5000 — see v0.4.2 pagination fix); `tf link <remote_key>` attaches a second machine to an existing remote account. Public profile cards (`/api/v1/card/{username}`) are rendered server-side SVG.

## Versioning & Release

Version is shared across Rust crates via `[workspace.package].version` in root `Cargo.toml`. When bumping, also update `CHANGELOG.md` (project uses a mix of English and Russian entries — follow existing style per-entry). Tags are `vX.Y.Z`; releases are cut with `gh release` and include the CHANGELOG section.

## Conventions

- Claude Code hooks integration lives in `contrib/claude-code/timeforged-heartbeat.sh` — the hook must remain non-blocking (fires-and-forgets via background `curl`) to avoid delaying user prompts.
- Rust edition: **2024** (workspace-wide).
- SQLite migrations are applied via `include_str!` + `sqlx::raw_sql` at startup; `002_public_profile.sql` uses `.ok()` because it adds a column that may already exist. New migrations should follow the same additive, idempotent pattern.
