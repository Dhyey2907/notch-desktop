// What the mouse is doing around the walking pet: a slow back-and-forth over it is petting, a
// fast swipe through it makes it dizzy, and a cursor resting near it on the taskbar is
// something to sneak up on. Pure, so it's tested.

export interface Sample {
  x: number;
  y: number;
  /** Milliseconds. */
  t: number;
}

/** How far back the gestures look. */
export const WINDOW_MS = 1500;
/** Faster than this through the pet is a swipe (px/s). */
const SWIPE_SPEED = 1400;
/** Petting is slower than this on average (px/s). */
const PAT_MAX_SPEED = 900;
/** A turn counts after at least this much travel. */
const PAT_MIN_TRAVEL = 6;
const PAT_TURNS = 3;
/** Resting: moved less than this… */
const REST_PX = 5;
/** …for this long. */
export const REST_MS = 2000;

export type Gesture = "pat" | "swipe" | null;

/** Keep only the recent samples. */
export function recent(samples: Sample[], now: number, ms = WINDOW_MS): Sample[] {
  return samples.filter((s) => now - s.t <= ms);
}

/** Pet or swipe, from the pointer's recent samples over the pet. */
export function classify(samples: Sample[]): Gesture {
  if (samples.length < 2) return null;
  let distance = 0;
  for (let i = 1; i < samples.length; i++) {
    const a = samples[i - 1];
    const b = samples[i];
    const dt = (b.t - a.t) / 1000;
    const d = Math.hypot(b.x - a.x, b.y - a.y);
    distance += d;
    if (dt > 0 && d / dt > SWIPE_SPEED && d > 20) return "swipe";
  }
  // Turns: horizontal direction changes after real travel.
  let turns = 0;
  let dir = 0;
  let travel = 0;
  for (let i = 1; i < samples.length; i++) {
    const dx = samples[i].x - samples[i - 1].x;
    if (dx === 0) continue;
    const d = Math.sign(dx);
    if (dir !== 0 && d !== dir && travel >= PAT_MIN_TRAVEL) {
      turns += 1;
      travel = 0;
    }
    if (d !== dir) travel = 0;
    dir = d;
    travel += Math.abs(dx);
  }
  const span = (samples[samples.length - 1].t - samples[0].t) / 1000;
  const speed = span > 0 ? distance / span : Infinity;
  return turns >= PAT_TURNS && speed < PAT_MAX_SPEED ? "pat" : null;
}

/** Where the cursor has been resting for a while (its x), or null. */
export function restingAt(samples: Sample[], now: number, ms = REST_MS): number | null {
  if (samples.length === 0) return null;
  const last = samples[samples.length - 1];
  // The newest sample must be fresh enough, and the history must reach back far enough.
  const since = samples.filter((s) => s.t >= now - ms);
  const before = samples.filter((s) => s.t < now - ms);
  if (before.length === 0) return null;
  const anchor = before[before.length - 1];
  const still = [anchor, ...since].every((s) => Math.hypot(s.x - last.x, s.y - last.y) <= REST_PX);
  return still ? last.x : null;
}

/** Whether a point is on the pet (plus a little slop: it's small and it moves). */
export function onPet(
  p: { x: number; y: number },
  rect: { left: number; right: number; top: number; bottom: number },
  slop = 6,
): boolean {
  return (
    p.x >= rect.left - slop && p.x <= rect.right + slop && p.y >= rect.top - slop && p.y <= rect.bottom + slop
  );
}

/**
 * A fast pass across the pet. The mouse is only sampled every few dozen milliseconds, so a quick
 * swipe can jump right over the small pet: look for a fast segment whose path crosses it.
 */
export function swipedThrough(
  samples: Sample[],
  rect: { left: number; right: number; top: number; bottom: number },
): boolean {
  for (let i = 1; i < samples.length; i++) {
    const a = samples[i - 1];
    const b = samples[i];
    const dt = (b.t - a.t) / 1000;
    if (dt <= 0 || dt > 0.12) continue;
    const speed = Math.hypot(b.x - a.x, b.y - a.y) / dt;
    if (speed <= SWIPE_SPEED) continue;
    const crossesX = Math.min(a.x, b.x) <= rect.right && Math.max(a.x, b.x) >= rect.left;
    const nearY = Math.max(a.y, b.y) >= rect.top - 12 && Math.min(a.y, b.y) <= rect.bottom + 12;
    if (crossesX && nearY) return true;
  }
  return false;
}
