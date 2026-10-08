# Overline — Architecture

## 1. Product summary

Overline is a Windows desktop companion that presents a small, persistent Dynamic Island–style surface near the top edge of the screen. It starts with Windows, stays quiet while idle, and expands into focused modules when the user interacts with it or when an event needs attention.

The product is a system layer for notifications, media, calendar, development activity, and assistant usage—not a conventional full-screen dashboard. Its foundation must let new modules be added without coupling them to the shell or to one another.

## 2. Architectural principles

- **Shell + Event Bus + Modules + Adapters:** keep the desktop shell, shared event flow, feature modules, and external integrations separate.
- **Quiet by default:** the collapsed island is compact; detailed content appears on demand or for a relevant live event.
- **Modules own their features:** each module owns its UI, state, actions, and event handling behind a stable contract.
- **Adapters isolate services:** Windows APIs and third-party services are accessed through adapters, not directly from module UI.
- **Local-first and resilient:** the shell and local features should remain usable when a service is offline or disconnected.
- **Explicit user control:** connections, startup behavior, notification permissions, and consequential actions are visible and controllable.

## 3. High-level system

```text
┌─────────────────────────────────────────────────────────┐
│                     Windows Desktop                     │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │                 Overline Shell               │  │
│  │  Window / positioning / animation / navigation     │  │
│  │  collapsed island / expanded module host           │  │
│  └──────────────────────┬────────────────────────────┘  │
│                         │ Module contracts              │
│  ┌──────────────────────┴────────────────────────────┐  │
│  │ Calendar │ Media │ Notifications │ Git │ AI Usage  │  │
│  │              Future modules and actions           │  │
│  └──────────────────────┬────────────────────────────┘  │
│                         │ typed app events               │
│  ┌──────────────────────┴────────────────────────────┐  │
│  │ Event Bus │ State Store │ Settings │ Permissions   │  │
│  └──────────────────────┬────────────────────────────┘  │
│                         │ ports / adapter interfaces     │
│  ┌──────────────────────┴────────────────────────────┐  │
│  │ Windows adapters │ Service adapters │ Persistence  │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Main layers

1. **Desktop shell** — owns the transparent/frameless window, screen placement, expand/collapse behavior, focus, input, animation, startup lifecycle, and module navigation. It does not implement service-specific business logic.
2. **Module host** — discovers and mounts enabled modules, routes navigation, and provides shared capabilities such as theme, settings access, and event subscription.
3. **Feature modules** — own their presentation and feature-specific state. Initial modules are Media, Calendar, Git, Claude/AI usage, Live Activities/approvals, and Phone Notifications.
4. **Application services** — coordinate cross-module behavior, typed events, notification prioritization, connection state, and settings.
5. **Adapters** — implement ports for Windows integration, calendar providers, media sessions, Git providers/local repositories, assistant usage sources, and phone notification bridges.
6. **Persistence** — stores user settings, module preferences, cached non-sensitive display data, and connection metadata using an appropriate local store. Secrets belong in the Windows credential store, not plain settings files.

## 4. Shell and interaction model

The shell has three presentation states:

- **Collapsed:** a small island with a concise status or active indicator.
- **Peek:** a short-lived preview on hover, click, or a live event.
- **Expanded:** the selected module or event surface, with clear dismissal and navigation.

The shell manages transitions and window behavior. Modules provide content for these states through the module contract. Live events may request a peek, but notification priority and user settings decide whether it is shown. The shell must respect display bounds, scaling, multi-monitor placement, and reduced-motion preferences.

## 5. Module contract

Each module should provide:

- Stable module identifier, display name, icon, and navigation metadata.
- Lifecycle hooks for initialization, activation, deactivation, and disposal.
- A compact/collapsed contribution where applicable and an expanded view.
- Typed subscriptions to relevant application events.
- User-facing actions expressed through application services or adapter ports.
- Module settings schema and connection/permission state.
- Loading, empty, disconnected, and error states.

Modules must not reach into another module's internal state. Cross-module coordination happens through typed events or shared application services. A module can be disabled without preventing the shell or other modules from starting.

## 6. Event flow

The event bus carries typed, versionable application events. Representative events include:

- `MediaPlaybackChanged`, `MediaTrackChanged`
- `CalendarConnectionChanged`, `UpcomingEventChanged`, `ReminderDue`
- `NotificationReceived`, `NotificationActionRequested`
- `GitRepositoryChanged`, `GitOperationRequested`, `GitOperationCompleted`
- `AssistantUsageUpdated`, `AgentApprovalRequested`
- `ModuleConnectionChanged`, `IntegrationError`

Adapters translate provider-specific callbacks and payloads into these application events. The shell consumes presentation-level signals; feature modules consume domain-level events. Actions flow in the opposite direction through application services to adapters, with results returned as events or explicit operation results. The event bus should support cancellation, deduplication where needed, and diagnostic logging without recording sensitive message contents by default.

## 7. Initial modules and boundaries

### Media

Reads the current Windows media session through a media adapter and presents track, playback controls, and available artwork. Provider artwork/backgrounds are shown only when supplied by the active session or service. Playback controls go through the adapter; the module does not scrape service interfaces.

### Calendar

Uses a calendar provider adapter for events and reminders. It presents an agenda and supports a connect flow, upcoming-event summaries, and configurable reminders. Meeting join actions are provider-aware and use the event's available join link. Calendar permissions and connection state are managed explicitly.

### Git

Combines local repository discovery/status with optional remote-provider capabilities. It shows repository state and offers appropriate commit, push, pull, and open-repository actions. Git operations run through a controlled service with progress, result, and error reporting; credentials are held by the operating system or provider tooling.

### Claude and assistant usage

Provides assistant activity and usage summaries through supported data sources or user-configured integrations. Usage data source, refresh time, and unavailable state should be clear. The module must not imply live or exact quota data when the connected source only provides estimates or delayed data.

#### Usage dashboard visual direction

Use the supplied reference as the visual target for the expanded usage view:

- A near-black charcoal canvas with a restrained, low-contrast surface treatment.
- A compact top row with **Overview** and **Models** tabs, plus time filters such as **All**, **30d**, and **7d**.
- Six small rounded metric cards in a two-row grid. Example metrics: Sessions, Messages, Total tokens, Active days, Peak hour, and Favorite model.
- A dense daily activity heatmap beneath the cards. Use dark gray for quiet days and a stepped range of blue shades for activity intensity. Preserve square cells, tight spacing, and a clear grid rhythm.
- A short, muted comparison caption below the heatmap, such as a token comparison against the prior period or a selected model.
- Keep labels subdued and values prominent, with compact typography and minimal borders. Blue is the primary data accent; reserve orange for agent approval actions elsewhere in the product.
- Make the layout adapt to the available island width: retain readable metrics, allow the heatmap to scroll or reflow at narrow widths, and provide accessible labels/tooltips for each day and intensity value. Do not rely on color alone to communicate values.

The screenshot is a visual reference, not a data contract. Metric names and comparison text should reflect the actual connected source and selected range.

### Live Activities and agent approvals

Shows concise transient activity cards for relevant events, such as an agent waiting for permission to run a command. Actions such as approve, dismiss, or open terminal must be explicit, attributable, and routed through the source integration. The display itself does not silently execute commands.

### Phone Notifications

Receives phone notifications and, where the bridge and source app support it, exposes reply or action controls. A phone bridge adapter owns transport, pairing, and capability negotiation. Unsupported replies must be presented as unavailable rather than simulated. Message bodies should be treated as sensitive and excluded from routine diagnostic logs.

## 8. Adapter ports

Define interfaces around capabilities rather than vendor-specific UI. Initial ports may include:

- `MediaSessionPort`
- `CalendarProviderPort`
- `GitRepositoryPort` and `GitRemotePort`
- `AssistantUsagePort`
- `AgentApprovalPort`
- `PhoneNotificationPort`
- `StartupAndWindowPort`
- `SecureCredentialPort`

An adapter reports connection status and capabilities, translates external data into internal types, and maps requested actions back to its provider. Provider-specific models must not leak into the shell or unrelated modules.

## 9. State and data ownership

- The shell owns window and navigation state.
- Each module owns its transient view state and module-specific preferences.
- Application services own cross-module coordination and event policy.
- Adapters own provider sessions and translation state.
- The settings store owns non-secret user preferences and module configuration.
- The operating system credential store owns secrets and tokens where supported.

Cache only what is needed for responsive rendering and graceful reconnection. Give cached external data a freshness indicator where staleness could mislead. Provide clear disconnect and data-clearing behavior for integrations.

## 10. Startup and lifecycle

On Windows sign-in, the startup entry launches the shell, restores permitted preferences, initializes enabled modules, and connects adapters in the background. A failed adapter must not block the island from appearing. Shutdown and sign-out dispose subscriptions and provider sessions cleanly. Startup can be toggled in settings, and the app should expose a recoverable path if a module repeatedly fails.

## 11. Reliability and security

- Treat integration input as untrusted and validate it at adapter boundaries.
- Keep credentials out of logs, crash reports, and ordinary configuration files.
- Require confirmation or clear user intent for consequential operations such as Git writes or agent command approval.
- Apply least-privilege permissions and explain why each integration requests access.
- Bound retries and use backoff for reconnecting providers.
- Surface errors within the relevant module with a retry or disconnect path; isolate failures so the shell remains available.
- Make notification privacy and content previews configurable.
- Keep diagnostic events useful while avoiding collection of private message text, tokens, or secrets.

## 12. Observability

Use structured, privacy-conscious diagnostics for app lifecycle, module activation, adapter connection state, action outcomes, and recoverable errors. Include correlation identifiers for operations. Redact credentials and sensitive payloads. Diagnostics should be local by default and exportable by the user when troubleshooting.

## 13. Suggested implementation sequence

1. Build the Windows shell, startup lifecycle, collapsed/peek/expanded states, settings, and module host.
2. Add the typed event model and module contract with a mock module to validate extension boundaries.
3. Integrate Media using the Windows media-session adapter.
4. Add Calendar connection, agenda, and reminders.
5. Add Git status and controlled Git actions.
6. Add Claude/assistant usage, including the overview, model tabs, range filters, metric cards, and activity heatmap visual direction above.
7. Add Live Activities and approval flows.
8. Add the phone notification bridge and supported reply actions.
9. Harden startup, permissions, reconnection, privacy, accessibility, and multi-monitor behavior.

The order keeps the shell and integration boundaries stable before adding more external services. Provider availability and supported capabilities should be confirmed during implementation rather than assumed by the UI.
