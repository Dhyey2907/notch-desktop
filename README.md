# Notch Desktop

A Windows Dynamic Island–style system layer: a small persistent surface at the top of the screen that expands into media, calendar, Git, and coding-agent modules.

Source-of-truth design docs live in [`docs/`](docs/): [architecture](docs/architecture.md), [design](docs/design.md), and [plan](docs/plan.md).

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
```

## Layout

```
src-tauri/src/   commands/ db/ events/ error.rs logging.rs state.rs
src/core/        ipc.ts (typed commands), eventBus.ts, generated/ (ts-rs types)
docs/            architecture, design, and plan documents
```
