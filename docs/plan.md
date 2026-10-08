# Overline --- Implementation Plan

## 0. Objective

Build a Windows desktop Dynamic Island / notch system layer that starts
with Windows and provides:

1.  Dynamic notch shell
2.  Live notifications
3.  Music controls
4.  Calendar and reminders
5.  Git/GitHub activity
6.  Claude/AI activity
7.  Agent approval notifications
8.  Android phone notification bridge
9.  Extensible module architecture

Build the foundation before adding integrations.

------------------------------------------------------------------------

# Phase 1 --- Project Foundation

## Goal

Create a working Tauri + React + TypeScript Windows application.

### Tasks

-   [ ] Create Tauri 2 project
-   [ ] Configure React + TypeScript
-   [ ] Configure Vite
-   [ ] Configure Windows build
-   [ ] Add formatting/linting
-   [ ] Add environment configuration
-   [ ] Create base application shell
-   [ ] Create Rust command structure
-   [ ] Create event bus abstraction
-   [ ] Create SQLite layer
-   [ ] Add migration support
-   [ ] Add logging

### Acceptance criteria

-   Application launches
-   Application builds as Windows executable
-   SQLite initializes
-   Frontend can call a Rust command
-   Rust can emit an event to frontend

------------------------------------------------------------------------

# Phase 2 --- Overline Shell

## Goal

Build the core visual system before integrations.

### Tasks

-   [ ] Create transparent borderless window
-   [ ] Position at top-center
-   [ ] Add rounded notch
-   [ ] Add always-on-top behavior
-   [ ] Implement compact state
-   [ ] Implement peek state
-   [ ] Implement expanded state
-   [ ] Implement collapsing animation
-   [ ] Implement monitor detection
-   [ ] Implement DPI/scaling support
-   [ ] Add system tray
-   [ ] Add close/minimize-to-tray behavior

### Acceptance criteria

The application can sit at the top of Windows all day without feeling
like a normal app window.

------------------------------------------------------------------------

# Phase 3 --- Design System

## Goal

Implement the visual language before building modules.

### Tasks

-   [ ] Create CSS design tokens
-   [ ] Create typography system
-   [ ] Create surface components
-   [ ] Create pill component
-   [ ] Create icon button
-   [ ] Create module container
-   [ ] Create notification card
-   [ ] Create progress indicator
-   [ ] Create media control
-   [ ] Create bottom module dock
-   [ ] Implement shared animation primitives

### Acceptance criteria

New modules can be assembled from consistent primitives.

------------------------------------------------------------------------

# Phase 4 --- Event System

## Goal

Make every module communicate through a shared event model.

### Tasks

-   [ ] Implement typed event definitions
-   [ ] Implement event bus
-   [ ] Implement event priority
-   [ ] Implement event queue
-   [ ] Implement notification deduplication
-   [ ] Implement timeout behavior
-   [ ] Implement module subscriptions
-   [ ] Implement event history where necessary

### Acceptance criteria

A fake test event can produce a notification in the notch without any
real integration.

------------------------------------------------------------------------

# Phase 5 --- Live Activity Engine

## Goal

Create the system responsible for deciding what appears in the notch.

### Priority

``` text
CRITICAL
HIGH
NORMAL
LOW
```

### Tasks

-   [ ] Create activity manager
-   [ ] Add priority queue
-   [ ] Add active activity state
-   [ ] Add activity timeout
-   [ ] Add user-dismiss behavior
-   [ ] Add activity replacement
-   [ ] Add stacked notification behavior
-   [ ] Add persistent activities
-   [ ] Add actionable activities

### Acceptance criteria

Multiple simultaneous events behave predictably.

------------------------------------------------------------------------

# Phase 6 --- Media Module

## Goal

Make music playback feel native.

### Tasks

-   [ ] Integrate Windows media session APIs
-   [ ] Detect active media session
-   [ ] Read title
-   [ ] Read artist
-   [ ] Read album
-   [ ] Read artwork
-   [ ] Read playback state
-   [ ] Implement play/pause
-   [ ] Implement previous/next
-   [ ] Implement seek where supported
-   [ ] Build artwork background
-   [ ] Add media peek animation

### Acceptance criteria

Starting/stopping/changing a supported media player updates the notch.

------------------------------------------------------------------------

# Phase 7 --- Calendar Module

## Goal

Implement the calendar experience shown in the reference design.

### Tasks

-   [ ] Create calendar provider interface
-   [ ] Implement Google Calendar adapter
-   [ ] Implement Microsoft Calendar adapter
-   [ ] Implement OAuth flow
-   [ ] Store credentials securely
-   [ ] Cache calendar data
-   [ ] Build month view
-   [ ] Build agenda view
-   [ ] Build reminders
-   [ ] Detect upcoming events
-   [ ] Add meeting countdown
-   [ ] Add join action
-   [ ] Add notification before meetings

### Acceptance criteria

The user can connect a calendar and see today's schedule inside the
notch.

------------------------------------------------------------------------

# Phase 8 --- Git Module

## Goal

Create developer-focused Git telemetry and actions.

### Tasks

-   [ ] Detect Git repositories
-   [ ] Add repository picker
-   [ ] Read current branch
-   [ ] Read Git status
-   [ ] Detect changed files
-   [ ] Detect ahead/behind
-   [ ] Read recent commits
-   [ ] Implement commit action
-   [ ] Implement push action
-   [ ] Implement pull action
-   [ ] Add GitHub adapter
-   [ ] Add PR information
-   [ ] Add repository metadata
-   [ ] Build activity heatmap
-   [ ] Add confirmation for destructive operations

### Acceptance criteria

A local repository can be selected and its status is visible without
opening a Git GUI.

------------------------------------------------------------------------

# Phase 9 --- AI / Claude Module

## Goal

Surface coding-agent activity.

### Tasks

-   [ ] Create generic AI provider interface
-   [ ] Create Claude adapter
-   [ ] Detect supported Claude Code activity
-   [ ] Show active project
-   [ ] Show current activity
-   [ ] Show usage data when officially available
-   [ ] Clearly label estimates if necessary
-   [ ] Add provider connection state
-   [ ] Add Claude launch action

### Acceptance criteria

The module never invents usage information and gracefully handles
unavailable provider data.

------------------------------------------------------------------------

# Phase 10 --- Agent Approval System

## Goal

Make AI agents able to request user attention.

### Tasks

-   [ ] Create agent activity event
-   [ ] Parse command/action metadata
-   [ ] Display approval prompt
-   [ ] Add dismiss
-   [ ] Add terminal/open action
-   [ ] Add approve/deny architecture
-   [ ] Add session permissions
-   [ ] Add persistent permission policies
-   [ ] Add destructive-command warning
-   [ ] Add audit logging

### Acceptance criteria

An agent event such as:

``` text
Claude Code wants to run:
swift test
```

appears as an actionable live activity.

------------------------------------------------------------------------

# Phase 11 --- Android Companion

## Goal

Mirror Android notifications and support replies.

### Desktop

-   [ ] Create pairing screen
-   [ ] Generate pairing QR
-   [ ] Create secure device identity
-   [ ] Create WebSocket server/client
-   [ ] Authenticate paired device
-   [ ] Receive normalized notifications
-   [ ] Show live notification
-   [ ] Send reply commands
-   [ ] Send dismiss commands
-   [ ] Handle disconnect/reconnect

### Android

-   [ ] Create companion application
-   [ ] Implement Notification Listener Service
-   [ ] Request notification permission
-   [ ] Normalize notification events
-   [ ] Implement WebSocket transport
-   [ ] Implement secure pairing
-   [ ] Receive desktop actions
-   [ ] Execute notification replies
-   [ ] Handle connection lifecycle
-   [ ] Add foreground/background reliability strategy

### Acceptance criteria

A supported Android notification appears on Windows and a reply typed in
the notch is delivered back to the phone.

------------------------------------------------------------------------

# Phase 12 --- Notification Center

## Goal

Allow the user to review recent notifications.

### Tasks

-   [ ] Notification history
-   [ ] App grouping
-   [ ] Search
-   [ ] Mark read
-   [ ] Dismiss
-   [ ] Clear all
-   [ ] Per-app notification settings
-   [ ] Retention policy

Privacy rule:

Do not retain notification contents indefinitely.

------------------------------------------------------------------------

# Phase 13 --- Settings

## Goal

Make every integration independently configurable.

### Tasks

-   [ ] General settings
-   [ ] Startup setting
-   [ ] Appearance
-   [ ] Animation intensity
-   [ ] Module enable/disable
-   [ ] Phone pairing
-   [ ] Calendar accounts
-   [ ] Git settings
-   [ ] AI settings
-   [ ] Notification privacy
-   [ ] Keyboard shortcuts
-   [ ] Diagnostics

------------------------------------------------------------------------

# Phase 14 --- Startup and Reliability

## Goal

Make the application behave like a real Windows utility.

### Tasks

-   [ ] Windows startup registration
-   [ ] Start minimized/hidden
-   [ ] Background initialization
-   [ ] Crash recovery
-   [ ] Integration failure isolation
-   [ ] Automatic reconnect
-   [ ] Logging
-   [ ] Diagnostic page
-   [ ] Resource monitoring
-   [ ] Graceful shutdown

------------------------------------------------------------------------

# Phase 15 --- Performance

## Targets

The application should remain lightweight.

### Goals

-   Low idle CPU
-   Low idle RAM
-   No continuous high-frequency polling
-   Event-driven updates
-   Lazy-load module UI
-   Cache external data
-   Avoid unnecessary React re-renders
-   Dispose unused subscriptions

### Tasks

-   [ ] Profile startup
-   [ ] Profile idle CPU
-   [ ] Profile memory
-   [ ] Profile animations
-   [ ] Optimize event subscriptions
-   [ ] Optimize image/artwork handling
-   [ ] Test with many notifications
-   [ ] Test with multiple monitors

------------------------------------------------------------------------

# Phase 16 --- Security Review

### Tasks

-   [ ] Review OAuth token storage
-   [ ] Review phone pairing
-   [ ] Encrypt mobile communication
-   [ ] Remove secrets from logs
-   [ ] Review Git command execution
-   [ ] Review agent command execution
-   [ ] Add command confirmation
-   [ ] Add permission revocation
-   [ ] Test malicious notification payloads
-   [ ] Test malformed external data

------------------------------------------------------------------------

# Phase 17 --- Polish

## Tasks

-   [ ] Refine notch dimensions
-   [ ] Refine spring animations
-   [ ] Refine shadows
-   [ ] Refine typography
-   [ ] Refine module transitions
-   [ ] Add sound feedback only where appropriate
-   [ ] Improve empty states
-   [ ] Improve errors
-   [ ] Add onboarding
-   [ ] Add first-run experience

------------------------------------------------------------------------

# Phase 18 --- Release

### Tasks

-   [ ] Production build
-   [ ] Windows installer
-   [ ] App icon
-   [ ] Auto-start verification
-   [ ] Clean install test
-   [ ] Upgrade test
-   [ ] Uninstall test
-   [ ] Permission revocation test
-   [ ] Multi-monitor test
-   [ ] Windows scaling test
-   [ ] Documentation

------------------------------------------------------------------------

# MVP Definition

Do NOT attempt every integration immediately.

The first impressive MVP should contain:

``` text
1. Overline shell
2. Animation system
3. Media controls
4. Fake/live activity engine
5. Calendar
6. Git
7. Claude activity
8. Settings
```

Phone integration should follow once the desktop architecture is stable.

------------------------------------------------------------------------

# Development Rules for Claude

## Rule 1 --- Read the docs first

Before changing code, read:

``` text
architecture.md
design.md
plan.md
```

Treat them as project source-of-truth documents.

If implementation conflicts with these documents, identify the conflict
before making a major architectural change.

## Rule 2 --- Do not over-engineer

Do not introduce microservices, unnecessary databases, message brokers,
or complicated abstractions.

The application is a local desktop utility.

## Rule 3 --- Build vertically

Prefer completing a small end-to-end feature over creating ten
unfinished subsystems.

Example:

``` text
Media adapter
 → Event
 → Activity manager
 → Overline UI
 → Controls
```

before starting another module.

## Rule 4 --- Keep integrations isolated

External integrations belong behind adapters.

Never put Google, GitHub, Android, Claude, or Windows API logic directly
inside visual components.

## Rule 5 --- No fake functionality

If an integration is not implemented, show an explicit placeholder
state.

Do not fabricate:

-   Usage statistics
-   Calendar events
-   Git data
-   Notification contents
-   Provider status

## Rule 6 --- Preserve the visual system

New features must use existing design tokens and primitives.

Do not introduce random colors, radii, shadows, typography, or animation
styles.

## Rule 7 --- Performance matters

This is a persistent desktop application.

Do not solve problems by adding aggressive polling.

Prefer:

``` text
event listener
→ state update
→ UI render
```

over:

``` text
setInterval → request → request → request
```

## Rule 8 --- Security before convenience

Anything capable of executing commands, replying to messages, or
accessing credentials must have an explicit permission model.

## Rule 9 --- Test each module independently

Every integration must be testable without the entire product being
connected.

Use mock adapters for development.

## Rule 10 --- Never rewrite working architecture casually

Before replacing an existing subsystem, explain:

-   why it is insufficient
-   what will change
-   what will remain compatible

------------------------------------------------------------------------

# Suggested Build Order

``` text
FOUNDATION
    ↓
NOTCH SHELL
    ↓
DESIGN SYSTEM
    ↓
EVENT BUS
    ↓
ACTIVITY ENGINE
    ↓
MEDIA
    ↓
CALENDAR
    ↓
GIT
    ↓
AI / CLAUDE
    ↓
AGENT APPROVALS
    ↓
ANDROID BRIDGE
    ↓
NOTIFICATION CENTER
    ↓
SETTINGS
    ↓
PERFORMANCE
    ↓
SECURITY
    ↓
POLISH
    ↓
RELEASE
```

The core philosophy is simple:

> Build the system that makes modules easy to build, then build the
> modules.
