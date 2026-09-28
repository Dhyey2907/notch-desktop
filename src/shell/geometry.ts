// Island dimensions per presentation state (logical px). The OS window is the island
// plus a margin for its shadow, and is resized per state so it never blocks the
// desktop beyond what is visible.

export type IslandMode = "collapsed" | "peek" | "expanded";

export interface Size {
  width: number;
  height: number;
}

export const ISLAND_SIZE: Record<IslandMode, Size> = {
  collapsed: { width: 184, height: 30 },
  peek: { width: 340, height: 64 },
  expanded: { width: 540, height: 300 },
};

/**
 * Room around the island for `--shadow-island` (8px offset + 18px blur). Keep it small:
 * the transparent margin still receives clicks, so it must not cover much of the desktop.
 */
export const WINDOW_MARGIN = { x: 22, top: 6, bottom: 30 } as const;

/** Module dock shown beneath the expanded island (design.md §17). */
export const DOCK = { height: 40, gap: 8 } as const;

export function windowSizeFor(mode: IslandMode): Size {
  const island = ISLAND_SIZE[mode];
  const dock = mode === "expanded" ? DOCK.gap + DOCK.height : 0;
  return {
    width: island.width + WINDOW_MARGIN.x * 2,
    height: island.height + dock + WINDOW_MARGIN.top + WINDOW_MARGIN.bottom,
  };
}

/** Smallest window that contains both sizes, used while animating between states. */
export function unionSize(a: Size, b: Size): Size {
  return { width: Math.max(a.width, b.width), height: Math.max(a.height, b.height) };
}

export function sameSize(a: Size | null, b: Size): boolean {
  return a !== null && a.width === b.width && a.height === b.height;
}
