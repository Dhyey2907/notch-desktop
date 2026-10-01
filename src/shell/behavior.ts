// How the notch behaves (Settings → Behavior, Shortcuts & gestures): pure rules the island
// and activity queue read from settings.
import type { FullscreenBehavior } from "../core/generated/FullscreenBehavior";
import type { QuietHours } from "../core/generated/QuietHours";
import type { Speed } from "../core/generated/Speed";

/** Hover must linger this long before the preview shows (design.md §18: no accidental panels). */
export const PEEK_DELAY_MS: Record<Speed, number> = { fast: 60, normal: 140, slow: 450 };
/** Hover-to-open waits longer than a peek, so passing through doesn't open the notch. */
export const OPEN_DELAY_MS: Record<Speed, number> = { fast: 180, normal: 320, slow: 650 };
/** "Open on hover" without an auto-close time still closes when the pointer leaves. */
export const HOVER_CLOSE_MS = 700;
/** How long notifications stay, relative to their default. */
export const CARD_TIME_SCALE: Record<Speed, number> = { fast: 0.6, normal: 1, slow: 1.8 };
/** Swipe distance needed: a sensitive ("fast") touchpad needs less. */
export const SWIPE_DISTANCE: Record<Speed, number> = { fast: 35, normal: 60, slow: 100 };

/** Whether `date` falls in quiet hours (which may run past midnight). */
export function inQuietHours(quiet: QuietHours, date: Date): boolean {
  if (!quiet.enabled || quiet.startMin === quiet.endMin) return false;
  const now = date.getHours() * 60 + date.getMinutes();
  return quiet.startMin < quiet.endMin
    ? now >= quiet.startMin && now < quiet.endMin
    : now >= quiet.startMin || now < quiet.endMin;
}

/** Should the notch hide over a fullscreen app right now? */
export function hideOverFullscreen(
  behavior: FullscreenBehavior,
  resting: boolean,
  activity: { priority: string } | null,
): boolean {
  if (behavior === "show" || !resting) return false;
  if (!activity) return true;
  // "Hide": only must-answer cards (approvals) come through.
  return behavior === "hide" && activity.priority !== "critical";
}

/** "22:00" ⇄ minutes after midnight, for time inputs. */
export function minutesToTime(min: number): string {
  return `${String(Math.floor(min / 60)).padStart(2, "0")}:${String(min % 60).padStart(2, "0")}`;
}

export function timeToMinutes(text: string): number | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(text);
  if (!m) return null;
  const h = Number(m[1]);
  const min = Number(m[2]);
  return h < 24 && min < 60 ? h * 60 + min : null;
}
