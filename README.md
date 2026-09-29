# Notch Desktop

A Windows Dynamic Island–style system layer: a small persistent surface at the top of the screen that expands into media, calendar, Git, and coding-agent modules.

Source-of-truth design docs live in [`docs/`](docs/): [architecture](docs/architecture.md), [design](docs/design.md), and [plan](docs/plan.md).

## Install

Download `Notch Desktop_<version>_x64-setup.exe` (or build it, below) and run it. It installs for the current user only — no admin prompt — and downloads WebView2 if it's missing. The notch appears at the top of your primary screen; **Ctrl+Alt+N** opens it, and the tray icon has Settings and Quit.

Uninstalling removes the app and its "Start with Windows" entry. Your settings stay in `%APPDATA%\com.notchdesktop.app` in case you reinstall; delete that folder to remove them too. Disconnect Google, Spotify, and Claude Code in Settings before uninstalling to also remove their stored credentials.

## Modules

| Module      | Source                                                                                          | Setup                                                                     |
| ----------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Media       | Windows media sessions (any app that shows in the volume flyout)                                | None                                                                      |
| Spotify     | Spotify Web API — pinned playlists (Premium needed to start playback)                           | Settings → Media: paste your own Spotify app's client ID                  |
| Calendar    | Google Calendar (read-only)                                                                     | Settings → Calendar: paste your own Google "Desktop app" OAuth client     |
| Git         | `git` CLI in repos you pick; pull requests via the `gh` CLI                                     | Settings → Git: add repositories; `gh auth login` for PRs                 |
| Claude Code | Usage from local transcripts; live activity and permission approvals via Claude Code HTTP hooks | Settings → AI: **Connect Claude Code** (shows exactly what it adds first) |

Everything runs locally. Secrets (OAuth refresh tokens, client secrets, the hook token) live in Windows Credential Manager, never in files, SQLite, or logs.

## Stack

Tauri 2 (Rust) · React 19 + TypeScript · Vite · Zustand · Motion · SQLite (rusqlite)

## Develop

Requires Node 22+, pnpm, Rust (MSVC toolchain), VS 2022 Build Tools, and WebView2.

```bash
pnpm install
pnpm tauri dev
```

| Script             | What it does                                               |
| ------------------ | ---------------------------------------------------------- |
| `pnpm check`       | Typecheck, ESLint, Prettier check, Vitest                  |
| `pnpm test:rust`   | Rust unit tests (also regenerates TS bindings)             |
| `pnpm clippy`      | Clippy with warnings as errors                             |
| `pnpm gen:types`   | Regenerate `src/core/generated/` from Rust types via ts-rs |
| `pnpm format`      | Prettier + rustfmt                                         |
| `pnpm tauri build` | Release build                                              |

Copy `.env.example` for local settings. Secrets never go in env files or SQLite; they belong in Windows Credential Manager.

### Driving the webview from scripts

Start the app with WebView2 remote debugging on, then evaluate JavaScript in the page:

```bash
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 pnpm tauri dev
node scripts/cdp-eval.mjs "document.title"
node scripts/cdp-screenshot.mjs page.png                        # webview render only
powershell -ExecutionPolicy Bypass -File scripts/window-rect.ps1                # window bounds, topmost/tool-window flags
powershell -ExecutionPolicy Bypass -File scripts/screen-capture.ps1 screen.png  # real screen pixels (true transparency)
```

## Layout

```
src-tauri/src/     commands/ (thin IPC layer), db/ (migrations), events/, settings/, secrets.rs,
                   shell/ (placement, tray, display changes), oauth.rs (PKCE + loopback),
                   media/ calendar/ spotify/ git/ claude/ (one service + adapters per integration)
src-tauri/installer/  NSIS hooks
src/core/          ipc.ts (typed commands), eventBus.ts, generated/ (ts-rs types)
src/design/        tokens.css (design.md tokens) and shared components
src/shell/         Island (collapsed/peek/activity/expanded), dock, module host
src/activity/      Live activity engine (priorities, timeouts, replacement)
src/modules/       home, media, calendar, git, claude (+ dev-only gallery and dev events)
src/settings/      The settings window (index.html#settings)
docs/              architecture, design, and plan documents
```

## Development mocks

Debug builds can replace integrations with badged sample data: `NOTCH_MOCK=media,calendar,git pnpm tauri dev`. Mocks are compiled out of release builds. The dev-only "Dev events" module fires test activities and fake Claude Code permission requests through the real pipeline.
