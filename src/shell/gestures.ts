// Two-finger touchpad swipes (and mouse wheel) on the island. Movement adds up until it
// crosses a threshold on one axis; then that swipe fires once and a short cooldown follows,
// so one flick is one action.

export type Swipe = "left" | "right" | "up" | "down";

export interface SwipeState {
  dx: number;
  dy: number;
  lockedUntil: number;
}

export const SWIPE_THRESHOLD = 60;
export const SWIPE_COOLDOWN_MS = 350;
/** Movement older than this no longer counts toward a swipe. */
const SETTLE_MS = 250;

export const idleSwipe = (): SwipeState & { lastAt: number } => ({ dx: 0, dy: 0, lockedUntil: 0, lastAt: 0 });

/**
 * Add one wheel event. Returns the swipe that fired (if any) and the next state.
 * Positive `deltaX` means "next" (as module switching already uses); positive `deltaY` is a
 * downward swipe.
 */
export function addWheel(
  state: SwipeState & { lastAt: number },
  deltaX: number,
  deltaY: number,
  now: number,
  threshold = SWIPE_THRESHOLD,
): { swipe: Swipe | null; state: SwipeState & { lastAt: number } } {
  if (now < state.lockedUntil) return { swipe: null, state };
  // A pause starts a new gesture.
  const fresh = now - state.lastAt > SETTLE_MS;
  const dx = (fresh ? 0 : state.dx) + deltaX;
  const dy = (fresh ? 0 : state.dy) + deltaY;
  const horizontal = Math.abs(dx) >= Math.abs(dy);
  const distance = horizontal ? dx : dy;
  if (Math.abs(distance) < threshold) {
    return { swipe: null, state: { dx, dy, lockedUntil: state.lockedUntil, lastAt: now } };
  }
  const swipe: Swipe = horizontal ? (dx > 0 ? "right" : "left") : dy > 0 ? "down" : "up";
  return { swipe, state: { dx: 0, dy: 0, lockedUntil: now + SWIPE_COOLDOWN_MS, lastAt: now } };
}
