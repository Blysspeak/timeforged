# Changelog

## [0.5.0] — 2026-04-01

### Added
- **Windows support** — full cross-platform daemon, CLI, and file watcher via `dirs` + `gethostname` crates
- **System tray app** (`timeforged-tray`) — icon in tray, tooltip with today's time + per-project breakdown, click opens dashboard
- **Windows window tracker** — `GetForegroundWindow` + `GetWindowTextW` via `windows-sys` crate
- **Windows installer** (`install.ps1`) — auto-installs Rust, Node.js, jq via winget/scoop/choco, configures Task Scheduler + Startup
- **Cross-platform installer** — `install.sh` auto-detects OS, installs all dependencies (Rust, Node, jq, GTK dev libs), supports both Linux and Windows (Git Bash)
- **Test suite** — 50 unit tests across all crates (config, models, util, debounce, window tracker, tray poller)

### Changed
- Config paths use `dirs` crate (`%APPDATA%` on Windows, `~/.config` on Linux) instead of hardcoded `$HOME/.config`
- Data paths use `dirs::data_dir()` (`%LOCALAPPDATA%` on Windows, `~/.local/share` on Linux)
- Hostname detection uses `gethostname` crate instead of `/etc/hostname`
- Window title path extraction supports Windows paths (`C:\...`)
- Error messages in CLI show platform-appropriate config paths
- Installer auto-installs all prerequisites (Rust, Node.js, jq, curl, GTK dev headers)
- Installer auto-adds `~/.local/bin` to shell profile PATH

## [0.4.2] — 2026-03-29

### Added
- **Auto-sync in daemon** — background task periodically pushes events to remote server (default 5min, configurable via `sync_interval` in config.toml) (e6c7115)

### Fixed
- **CLI sync pagination** — client requested limit=50000 while server caps at 5000, breaking multi-page sync (e6c7115)

## [0.4.1] — 2026-03-16

### Security
- **Fix API key leakage** — auto-inject now only when daemon binds to localhost, prevents key exposure on public 0.0.0.0 deployments
- **Security headers** — X-Frame-Options: DENY, X-Content-Type-Options: nosniff, X-XSS-Protection, Referrer-Policy
- **Event rate limiting** — 120 write requests/min/IP on event endpoints to prevent spam

## [0.4.0] — 2026-03-16

### Added
- **Remote sync** — sync local events to `timeforged.blysspeak.space` with unlimited pagination (no more 5000 event cap)
- **Public profile cards** — SVG activity cards for GitHub READMEs at `/api/v1/card/{username}`
- **User registration** — `POST /api/v1/register` with rate limiting
- **Dashboard period switcher** — Today / 7 Days / 30 Days / All Time tabs
- **All Time stats** — always-visible card showing cumulative hours, active days, and project count
- **Dashboard auto-auth** — API key auto-injected from `cli.toml` on localhost, no manual setup
- **API key management UI** — Settings page now has connect/disconnect for API key
- **CORS** — added `timeforged.blysspeak.space` origin

### Changed
- Installer default remote URL updated to `https://timeforged.blysspeak.space`
- Projects and languages lists now reflect the selected period
- Sync uses paginated loop with batches of 100 instead of single 5000-capped request

### Fixed
- Sync state properly resets when switching remote servers

## [0.3.0] — 2026-03-15

### Added
- Remote sync (`tf sync`) with auto-sync timer (every 15 min)
- Public profile cards (SVG)
- Full installer script with systemd, Claude Code hooks, Waybar module
- Web dashboard with activity chart, projects, languages, sessions
- File watcher with inotify, debounce, git branch detection
- Window tracker (hyprctl/xdotool)
- Rate-limited registration endpoint
- Docker support (Dockerfile + docker-compose)
