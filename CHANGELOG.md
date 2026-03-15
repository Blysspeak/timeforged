# Changelog

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
