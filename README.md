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
| Git         | `git` CLI in repos you pick; pull requests via Continue with GitHub or the `gh` CLI             | Settings → Git: add repositories; `gh auth login` for PRs                 |
| Claude Code | Usage from local transcripts; live activity and permission approvals via Claude Code HTTP hooks | Settings → AI: **Connect Claude Code** (shows exactly what it adds first) |

Everything runs locally. Secrets (OAuth refresh tokens, client secrets, the hook token) live in Windows Credential Manager, never in files, SQLite, or logs.

## One-click sign-in ("Continue with Google / Spotify / GitHub")

Those buttons are OAuth sign-ins. The app is registered once with each provider, and that registration's **client ID** is built into the app. Clicking the button opens the provider's own sign-in page in your browser; when you approve, the provider hands Notch a token, which it keeps in Windows Credential Manager. Google and Spotify send you back to Notch through a local `127.0.0.1` address (with PKCE). GitHub uses the "device code" flow, the same one the GitHub CLI uses: Notch shows a code, and you enter it on github.com.

Notch has no client of its own until you register one. To turn the buttons on, copy `src-tauri/oauth-clients.example.json` to `src-tauri/oauth-clients.json` (gitignored, so it never reaches the repo), fill in what you register below, and rebuild. Leave a provider's placeholder in place to skip it. Without the file, Settings shows the "paste your own client ID" setup instead.

- **Google** (console.cloud.google.com):
  1. Create a project and enable the **Google Calendar API**.
  2. Configure the **OAuth consent screen** as _External_, and add yourself under **Test users**.
  3. Go to **Credentials → Create credentials → OAuth client ID**, choose **Desktop app**, and copy both the client ID and the secret. Google documents that a desktop app's secret can't be kept confidential.
- **Spotify** (developer.spotify.com/dashboard):
  1. **Create app**, adding the redirect URI `http://127.0.0.1:43821/callback`.
  2. Tick **Web API**.
  3. Copy the **Client ID**. No secret is needed.
- **GitHub** (github.com → Settings → Developer settings → OAuth Apps → **New OAuth App**):
  1. Use any homepage and callback URL (for example `https://github.com`).
  2. Tick **Enable Device Flow**.
  3. Copy the **Client ID**. No secret is needed.

Signing in to GitHub asks for the `repo` scope, which lets Notch list pull requests in your private repositories as well as public ones.

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
