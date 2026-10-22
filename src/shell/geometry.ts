// Island dimensions per presentation state (logical px). The OS window is the island
// plus a margin for its shadow, and is resized per state so it never blocks the
// desktop beyond what is visible.

export type IslandMode = "collapsed" | "peek" | "activity" | "expanded";

export interface Size {
  width: number;
  height: number;
}

/** Default island size per mode. Activities size themselves via `activityIslandSize`. */
export const ISLAND_SIZE: Record<IslandMode, Size> = {
  // Resting bubble: small enough to sit above browser tabs without covering them.
  collapsed: { width: 64, height: 14 },
  peek: { width: 400, height: 64 },
  activity: { width: 380, height: 72 },
  expanded: { width: 540, height: 300 },
};

/** A live window view (Desk): 16:10 with a title bar. */
export const MINI_SIZE: Size = { width: 440, height: 290 };

/** Game mode readout: one slim row. */
export const GAME_HUD_SIZE: Size = { width: 470, height: 36 };

/** Volume / brightness HUD: one slim row. */
export const HUD_SIZE: Size = { width: 300, height: 44 };

/**
 * Island size for an activity: taller when it has a row of text buttons, wider when it has
 * inline icon controls, and wider and taller still when it shows an exact command.
 */
export function activityIslandSize(actions: "none" | "inline" | "row", code = false): Size {
  if (code) return { width: 480, height: actions === "row" ? 212 : 160 };
  if (actions === "row") return { width: 420, height: 136 };
  if (actions === "inline") return { width: 440, height: 72 };
  return ISLAND_SIZE.activity;
}

/**
 * Room around the island for `--shadow-island` (8px offset + 18px blur). Keep it small:
 * the transparent margin still receives clicks, so it must not cover much of the desktop.
 */
export const WINDOW_MARGIN = { x: 22, top: 6, bottom: 30 } as const;

/** Module dock shown beneath the expanded island (design.md §17). */
export const DOCK = { height: 40, gap: 8 } as const;

/**
 * The window for a state. `dockWidth`: the dock's natural width; when expanded the window grows
 * to fit it, so a long dock is never cut off at the ends.
 */
export function windowSizeFor(mode: IslandMode, island: Size = ISLAND_SIZE[mode], dockWidth = 0): Size {
  // At rest the window is exactly the bubble: no shadow margin, so the transparent area
  // around it never swallows clicks meant for the apps underneath (e.g. browser tabs).
  if (mode === "collapsed") return { width: island.width, height: island.height + WINDOW_MARGIN.top };
  const dock = mode === "expanded" ? DOCK.gap + DOCK.height : 0;
  const content = mode === "expanded" ? Math.max(island.width, dockWidth) : island.width;
  return {
    width: content + WINDOW_MARGIN.x * 2,
    height: island.height + dock + WINDOW_MARGIN.top + WINDOW_MARGIN.bottom,
  };
}

/** Pill radius for short islands, a fixed corner once content gets taller (design.md §6). */
export function islandRadius(size: Size): number {
  const short = Math.min(size.width, size.height);
  return short <= 72 ? short / 2 : 24;
}

/** Smallest window that contains both sizes, used while animating between states. */
export function unionSize(a: Size, b: Size): Size {
  return { width: Math.max(a.width, b.width), height: Math.max(a.height, b.height) };
}

export function sameSize(a: Size | null, b: Size): boolean {
  return a !== null && a.width === b.width && a.height === b.height;
}
