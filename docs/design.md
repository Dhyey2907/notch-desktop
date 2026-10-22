# Overline --- Design System

## 1. Design Direction

The product should feel like a premium desktop system surface.

Reference characteristics from the provided designs:

-   Minimal
-   Dark
-   High contrast
-   Soft rounded geometry
-   Compact information density
-   Large negative space
-   Subtle shadows
-   Carefully controlled motion
-   White/light neutral surrounding environment in marketing visuals
-   Near-black interface
-   Small accent colors used only to communicate state

Do not copy another product's exact interface.

The goal is:

> "Everything important on my computer appears here without making me
> open another app."

------------------------------------------------------------------------

## 2. Core Visual Concept

The notch is a small black pill at the top center.

Collapsed:

``` text
       ╭────────────────────────────╮
       │   ●     ♪     ◌     ⋯     │
       ╰────────────────────────────╯
```

It should be visually quiet when nothing is happening.

Expanded:

``` text
       ╭──────────────────────────────────────╮
       │                                      │
       │            Module content            │
       │                                      │
       ╰──────────────────────────────────────╯
```

The island expands horizontally and vertically with spring animation.

It must feel like the interface is physically unfolding from the notch.

------------------------------------------------------------------------

## 3. Visual Hierarchy

Priority order:

1.  Current event/action
2.  Primary information
3.  Primary action
4.  Supporting metadata
5.  Secondary controls

Never give every piece of information equal visual weight.

------------------------------------------------------------------------

## 4. Colors

Base:

``` css
--background: #090909;
--surface: #111111;
--surface-elevated: #171717;
--surface-hover: #202020;

--text-primary: #F5F5F5;
--text-secondary: #A3A3A3;
--text-muted: #666666;

--border: rgba(255,255,255,0.08);
--border-strong: rgba(255,255,255,0.14);
```

Accent colors are semantic:

``` text
Success     green
Warning     amber/orange
Danger      red
Info        blue
Active      white
```

Do not make the whole application colorful.

Accent color should indicate meaning, not decoration.

------------------------------------------------------------------------

## 5. Typography

Use a modern system-oriented sans-serif.

Preferred:

-   Inter
-   SF Pro equivalent if available
-   Segoe UI fallback

Weights:

-   400 body
-   500 labels
-   600 emphasis
-   700 headlines

Avoid excessive font sizes.

The interface should feel dense but breathable.

------------------------------------------------------------------------

## 6. Shape Language

Primary radius:

``` text
12px → small elements
16px → cards
20px → module surfaces
24px → expanded notch
999px → pills
```

No sharp rectangular dashboard cards unless required by content.

------------------------------------------------------------------------

## 7. Shadows

Use layered soft shadows rather than a single heavy shadow.

The notch should visually separate itself from the desktop without
looking like a floating browser window.

Example:

``` css
box-shadow:
  0 12px 40px rgba(0,0,0,.22),
  0 2px 8px rgba(0,0,0,.18);
```

------------------------------------------------------------------------

## 8. Motion

Motion is a core part of the product.

Principles:

-   Fast for tiny interactions
-   Smooth for expansion
-   Spring-like for the island
-   Never bouncy for serious notifications
-   Use opacity + scale + height together
-   Animate content after the container starts expanding

Approximate timing:

``` text
Icon interaction: 120–180ms
Peek: 180–250ms
Expansion: 350–500ms
Collapse: 250–400ms
Large content transition: 300–500ms
```

Avoid excessive animation.

------------------------------------------------------------------------

## 9. Idle State

Idle state is intentionally tiny.

Example:

``` text
        ┌─────────────────────────┐
        │  ▪   ♪   ◌   ⋯          │
        └─────────────────────────┘
```

Possible indicators:

-   Music playing
-   Unread notification count
-   Calendar event approaching
-   Agent activity
-   Recording/activity state

No persistent dashboard should occupy the screen.

------------------------------------------------------------------------

## 10. Notification Peek

A notification should briefly expand the island.

Example:

``` text
╭─────────────────────────────────────╮
│  M   Mansi                          │
│      Chal chal aave                 │
│                         now         │
╰─────────────────────────────────────╯
```

After a timeout:

``` text
→ compact notch
```

For multiple notifications, stack them vertically or show a count rather
than flooding the desktop.

------------------------------------------------------------------------

## 11. Messaging

Messaging module should resemble a live activity stream rather than a
chat application.

Example:

``` text
╭───────────────────────────────────────╮
│  M   Mansi                    now     │
│      Chal chal aave                   │
│                                       │
│  A   Arnav                    now     │
│      Lunch at 1?                      │
│                                       │
│  M   Maya                     now     │
│      Landed! See you soon             │
╰───────────────────────────────────────╯
```

Each notification may expose:

``` text
Reply
Mark read
Open app
Dismiss
```

Reply should happen inline.

------------------------------------------------------------------------

## 12. Music

Music is one of the most visual modules.

Use the current track's artwork as a subtle background layer.

Structure:

``` text
╭────────────────────────────────────────────╮
│                                            │
│  [ART]    Golden Hour Drive                │
│           Neon Harbor · Coastline          │
│                                            │
│           ━━━━━━━●────────                 │
│                                            │
│             ◀      ❚❚      ▶              │
│                                            │
╰────────────────────────────────────────────╯
```

Artwork treatment:

-   Blur heavily
-   Reduce opacity
-   Add dark gradient
-   Keep controls readable
-   Do not make the artwork overpower the content

------------------------------------------------------------------------

## 13. Calendar

Calendar expanded view should be split into two areas.

``` text
╭──────────────────────────────────────────────╮
│                                              │
│  September             Saturday, 26 Sept     │
│  ┌──────────────┐      3 reminders           │
│  │ calendar     │                             │
│  │      26      │      Today's agenda        │
│  └──────────────┘      ─────────────         │
│                         2:07  Launch          │
│                         7:07  PR #214         │
│                         8:30  Flights         │
│                                              │
╰──────────────────────────────────────────────╯
```

A meeting starting soon should transform the notch into a compact
countdown:

``` text
        ●  Design Review · 04:32
```

Then:

``` text
        [Join Meeting]
```

------------------------------------------------------------------------

## 14. Git Module

Git should feel like developer telemetry, not a full Git client.

Show:

-   Current repository
-   Branch
-   Modified files
-   Ahead/behind
-   Recent commits
-   PR activity
-   Activity heatmap
-   Current coding agent

Example:

``` text
╭──────────────────────────────────────────────╮
│ CODING                           Claude       │
│                                              │
│ my-project / main                            │
│                                              │
│ 12 changed files       ↑ 2 commits           │
│                                              │
│ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪                  │
│                                              │
│ [Commit] [Push] [Pull] [Open Repository]     │
╰──────────────────────────────────────────────╯
```

Destructive Git actions must require confirmation.

------------------------------------------------------------------------

## 15. AI / Claude Module

The AI module should surface useful context without pretending to be the
AI itself.

Example:

``` text
╭──────────────────────────────────────────────╮
│ CLAUDE CODE                                  │
│                                              │
│ Working in: notch-desktop                    │
│                                              │
│ Session usage                                │
│ ███████████████░░░  63%                      │
│                                              │
│ 48.8M tokens                                 │
│                                              │
│ [Open Claude]                                │
╰──────────────────────────────────────────────╯
```

If official usage is unavailable:

``` text
Usage unavailable
```

or clearly label local estimates.

Never fabricate provider statistics.

------------------------------------------------------------------------

## 16. Agent Approval

Approval notifications should be visually distinct.

``` text
╭──────────────────────────────────────────────╮
│ 🟠  CLAUDE CODE NEEDS YOU                    │
│                                              │
│ Wants to run                                 │
│                                              │
│     swift test                               │
│                                              │
│  [Dismiss]             [Open Terminal]        │
╰──────────────────────────────────────────────╯
```

For potentially destructive actions, use a stronger warning state and
show the exact command.

------------------------------------------------------------------------

## 17. Bottom Utility Dock

When a module is expanded, a small secondary dock may appear underneath.

Possible controls:

``` text
[Home] [Music] [Notifications] [Git] [AI]
[Calendar] [System] [Settings]
```

The dock should remain compact.

The currently active module receives a subtle raised/filled treatment.

------------------------------------------------------------------------

## 18. Interaction Model

### Hover

Hovering the notch:

``` text
Idle
 ↓
Subtle expansion
```

Do not automatically open a large panel just because the pointer passes
over it.

### Click

Clicking:

``` text
notch → active module
```

### Swipe / scroll

Where useful:

-   Horizontal → skip track on the closed notch (never switches module; panels scroll sideways)
-   Vertical → notification list

Keyboard accessibility must remain available.

------------------------------------------------------------------------

## 19. Responsive Behavior

Desktop-first.

Support:

-   100%
-   125%
-   150%
-   200% Windows scaling

The island must calculate its position from actual monitor/work-area
dimensions.

Do not hardcode pixel coordinates based on one display.

------------------------------------------------------------------------

## 20. Multi-Monitor

The island belongs to the active monitor.

Settings should eventually allow:

``` text
Primary monitor
Active monitor
All monitors
```

For MVP, use the primary monitor.

------------------------------------------------------------------------

## 21. Settings Design

Settings should not be a huge dashboard.

Sections:

``` text
General
Appearance
Modules
Phone
Calendar
Media
Git
AI
Notifications
Privacy
Shortcuts
About
```

Each module can be independently enabled/disabled.

------------------------------------------------------------------------

## 22. Empty States

Empty states should teach the user what to do.

Bad:

``` text
No calendar data.
```

Good:

``` text
Your day lives here.

Connect Google Calendar or Outlook
to see meetings and reminders.

[Connect Calendar]
```

------------------------------------------------------------------------

## 23. Error States

Errors must be calm.

Example:

``` text
Calendar disconnected

Reconnect your account to restore
meeting and reminder updates.

[Reconnect]
```

Never expose raw API errors to the main UI.

------------------------------------------------------------------------

## 24. Design Rule

The most important rule:

> The interface should reveal itself when needed and disappear when it
> is not.

The user should spend more time using the computer and less time
managing the dashboard.
