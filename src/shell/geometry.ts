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
 * `top` is the gap on the screen-edge side when the notch floats (see `edgeGap`).
 */
export const WINDOW_MARGIN = { x: 22, top: 6, bottom: 30 } as const;

/** Module dock shown beneath the expanded island (design.md §17). */
export const DOCK = { height: 40, gap: 8 } as const;

/** The concave curves where an attached notch meets the screen edge (logical px). */
export const SHOULDER = 10;

/**
 * Auto-hide: while the notch is tucked away, the window shrinks to a strip this thick along
 * the screen edge, `slack` px wider than the bubble on each end, so pointing at the edge
 * there brings it back. Thin, so it barely covers what's underneath.
 */
export const TUCKED_STRIP = { thickness: 2, slack: 80 } as const;

export type NotchEdge = "top" | "bottom" | "left" | "right";

let placement: { edge: NotchEdge; attached: boolean } = { edge: "top", attached: true };

/**
 * Where the notch sits and whether it hangs off the screen edge ("Distance from the edge" =
 * None) or floats a little away from it. Window sizes depend on it, so refit after a change.
 */
export function setNotchPlacement(edge: NotchEdge, attached: boolean) {
  placement = { edge, attached };
}

export function notchPlacement() {
  return placement;
}

/** Room between the screen edge and the island inside the window: none when attached. */
export function edgeGap(): number {
  return placement.attached ? 0 : WINDOW_MARGIN.top;
}

/** Whether the attached notch on the top or bottom edge gets shoulders. */
export function hasShoulders(): boolean {
  return placement.attached && (placement.edge === "top" || placement.edge === "bottom");
}

export interface WindowOptions {
  /** Auto-hide has tucked the resting bubble into the screen edge. */
  tucked?: boolean;
  /** The resting bubble stands upright on a side edge. */
  vertical?: boolean;
}

/**
 * The window for a state. `dockWidth`: the dock's natural width; when expanded the window grows
 * to fit it, so a long dock is never cut off at the ends.
 */
export function windowSizeFor(
  mode: IslandMode,
  island: Size = ISLAND_SIZE[mode],
  dockWidth = 0,
  { tucked = false, vertical = false }: WindowOptions = {},
): Size {
  if (mode === "collapsed" && tucked) {
    const { thickness, slack } = TUCKED_STRIP;
    return vertical
      ? { width: thickness, height: island.height + slack * 2 }
      : { width: island.width + slack * 2, height: thickness };
  }
  // At rest the window is exactly the bubble (plus its shoulders): no shadow margin, so the
  // transparent area around it never swallows clicks meant for the apps underneath.
  if (mode === "collapsed") {
    const shoulders = hasShoulders() ? SHOULDER * 2 : 0;
    return { width: island.width + shoulders, height: island.height + edgeGap() };
  }
  const dock = mode === "expanded" ? DOCK.gap + DOCK.height : 0;
  const content = mode === "expanded" ? Math.max(island.width, dockWidth) : island.width;
  return {
    width: content + WINDOW_MARGIN.x * 2,
    height: island.height + dock + edgeGap() + WINDOW_MARGIN.bottom,
  };
}

/**
 * Corner radii for the island as a CSS `border-radius` (top-left, top-right, bottom-right,
 * bottom-left). Attached, the corners against the screen edge are square so the notch reads
 * as part of the edge rather than floating near it.
 */
export function islandCorners(radius: number): string {
  const r = `${radius}px`;
  if (!placement.attached) return `${r} ${r} ${r} ${r}`;
  switch (placement.edge) {
    case "top":
      return `0px 0px ${r} ${r}`;
    case "bottom":
      return `${r} ${r} 0px 0px`;
    case "left":
      return `0px ${r} ${r} 0px`;
    case "right":
      return `${r} 0px 0px ${r}`;
  }
}

/** Where the island slides to when auto-hide tucks it into its edge. */
export function tuckOffset(size: Size): { x: number; y: number } {
  const away = 4;
  switch (placement.edge) {
    case "top":
      return { x: 0, y: -(size.height + away) };
    case "bottom":
      return { x: 0, y: size.height + away };
    case "left":
      return { x: -(size.width + away), y: 0 };
    case "right":
      return { x: size.width + away, y: 0 };
  }
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
