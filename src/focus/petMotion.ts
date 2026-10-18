// Where the walking pet is and what its body is doing, one frame at a time: a patrol along the
// taskbar, a run to a spot (a ball, your cursor), being carried, and falling back down with a
// bounce. Pure, so it's tested; the walker draws whatever this returns.

export type Mode = "walk" | "pause" | "run" | "held" | "fall" | "still";

export interface Body {
  /** Left edge, in the window's pixels. */
  x: number;
  /** Height above the taskbar (0 = standing on it). */
  y: number;
  /** Upward speed while falling (px/s). */
  vy: number;
  /** 1 = facing right. */
  dir: 1 | -1;
  mode: Mode;
  /** Where a run goes. */
  target: number | null;
  /** Seconds left of a pause at the end of the patrol. */
  pause: number;
}

export const PET_W = 30;
export const WALK_SPEED = 34;
export const SULK_SPEED = 16;
export const RUN_SPEED = 190;
const GRAVITY = 2200;
/** Faster than this when it lands, it bounces once. */
const BOUNCE_ABOVE = 260;
const BOUNCE_KEEP = 0.35;
const EDGE_PAUSE_S = 2.2;
/** How close counts as "there". */
const ARRIVE_PX = 3;

export function startBody(width: number): Body {
  return { x: Math.round(width * 0.04), y: 0, vy: 0, dir: 1, mode: "walk", target: null, pause: 0 };
}

export interface StepResult {
  body: Body;
  /** A run just reached its target. */
  arrived: boolean;
  /** It just came to rest after a fall. */
  landed: boolean;
}

/** Keep the pet inside the strip, a little in from the edges. */
export function clampX(x: number, width: number): number {
  const margin = Math.max(8, width * 0.02);
  return Math.min(Math.max(x, margin), Math.max(margin, width - margin - PET_W));
}

export function step(b: Body, dt: number, width: number, opts: { sulky?: boolean } = {}): StepResult {
  const next: Body = { ...b };
  let arrived = false;
  let landed = false;
  const lo = clampX(-Infinity, width);
  const hi = clampX(Infinity, width);
  switch (b.mode) {
    case "walk": {
      next.x = b.x + b.dir * (opts.sulky ? SULK_SPEED : WALK_SPEED) * dt;
      if (next.x >= hi && b.dir === 1) {
        next.x = hi;
        next.mode = "pause";
        next.pause = EDGE_PAUSE_S;
      } else if (next.x <= lo && b.dir === -1) {
        next.x = lo;
        next.mode = "pause";
        next.pause = EDGE_PAUSE_S;
      }
      break;
    }
    case "pause": {
      next.pause = b.pause - dt;
      if (next.pause <= 0) {
        next.pause = 0;
        next.mode = "walk";
        next.dir = b.x >= (lo + hi) / 2 ? -1 : 1;
      }
      break;
    }
    case "run": {
      const target = clampX(b.target ?? b.x, width);
      const gap = target - b.x;
      if (Math.abs(gap) <= ARRIVE_PX) {
        next.x = target;
        arrived = true;
        break;
      }
      next.dir = gap > 0 ? 1 : -1;
      const move = Math.min(Math.abs(gap), RUN_SPEED * dt);
      next.x = b.x + next.dir * move;
      if (Math.abs(target - next.x) <= ARRIVE_PX) {
        next.x = target;
        arrived = true;
      }
      break;
    }
    case "fall": {
      next.vy = b.vy - GRAVITY * dt;
      next.y = b.y + next.vy * dt;
      if (next.y <= 0) {
        next.y = 0;
        if (-next.vy > BOUNCE_ABOVE) {
          next.vy = -next.vy * BOUNCE_KEEP;
        } else {
          next.vy = 0;
          next.mode = "walk";
          landed = true;
        }
      }
      break;
    }
    case "held":
    case "still":
      break;
  }
  return { body: next, arrived, landed };
}

/** Run to `x` (a ball, your cursor). */
export function runTo(b: Body, x: number): Body {
  return { ...b, mode: "run", target: x, y: 0, vy: 0 };
}

/** Picked up: follow the pointer, feet up to `maxY` above the taskbar. */
export function carry(b: Body, x: number, y: number, width: number, maxY: number): Body {
  return { ...b, mode: "held", x: clampX(x, width), y: Math.min(Math.max(y, 0), maxY), vy: 0 };
}

/** Let go: it falls (and bounces) back onto the taskbar. */
export function drop(b: Body): Body {
  return { ...b, mode: b.y > 0 ? "fall" : "walk", vy: 0 };
}
