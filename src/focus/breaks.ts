// Healthy breaks: eye breaks (20-20-20), stretching and water, counted in time you're actually
// at the PC. Stepping away for a few minutes counts as a break. Reminders wait while you focus,
// play, sit in a meeting or have quiet hours, and come once that ends.
import type { BreakSettings } from "../core/generated/BreakSettings";

export type BreakKind = "eyes" | "stretch" | "water";

/** Away this long (no keyboard or mouse) resets every timer: that was a break. */
export const NATURAL_BREAK_MS = 5 * 60_000;
/** Breaks needed in a day for it to count toward the streak. */
export const STREAK_BREAKS = 3;

export type Timers = Record<BreakKind, number>;

export const ZERO: Timers = { eyes: 0, stretch: 0, water: 0 };

export function intervalMs(settings: BreakSettings, kind: BreakKind): number | null {
  const on = { eyes: !settings.noEyes, stretch: !settings.noStretch, water: !settings.noWater }[kind];
  const minutes = { eyes: settings.eyeMin, stretch: settings.stretchMin, water: settings.waterMin }[kind];
  return on && minutes > 0 ? minutes * 60_000 : null;
}

/**
 * Advance the timers by `stepMs` of use. Returns the new timers and the reminder that's due
 * (at most one at a time; the eyes first). `held`: focus, a game, a meeting or quiet hours.
 */
export function tick(
  timers: Timers,
  settings: BreakSettings,
  { stepMs, idleMs, held }: { stepMs: number; idleMs: number; held: boolean },
): { timers: Timers; due: BreakKind | null } {
  if (idleMs >= NATURAL_BREAK_MS) return { timers: ZERO, due: null };
  // Only time at the PC counts.
  const used = idleMs < 60_000 ? stepMs : 0;
  const next: Timers = {
    eyes: timers.eyes + used,
    stretch: timers.stretch + used,
    water: timers.water + used,
  };
  if (held) return { timers: next, due: null };
  for (const kind of ["eyes", "stretch", "water"] as const) {
    const every = intervalMs(settings, kind);
    if (every !== null && next[kind] >= every) return { timers: next, due: kind };
  }
  return { timers: next, due: null };
}

/** Restart one timer (the break was taken, skipped, or snoozed by `snoozeMs`). */
export function reset(timers: Timers, settings: BreakSettings, kind: BreakKind, snoozeMs = 0): Timers {
  const every = intervalMs(settings, kind) ?? 0;
  return { ...timers, [kind]: snoozeMs > 0 ? Math.max(0, every - snoozeMs) : 0 };
}

/** "2026-10-02" in local time. */
export function dayKey(date: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${p(date.getMonth() + 1)}-${p(date.getDate())}`;
}

/** Count a break taken today; a day with three breaks extends the streak. */
export function recordBreak(s: BreakSettings, now: Date): Partial<BreakSettings> {
  const today = dayKey(now);
  const yesterday = dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1));
  const count = s.day === today ? s.count + 1 : 1;
  const change: Partial<BreakSettings> = { day: today, count };
  if (count === STREAK_BREAKS) {
    const streak = s.countedDay === yesterday ? s.streak + 1 : 1;
    Object.assign(change, { streak, countedDay: today, best: Math.max(s.best, streak) });
  }
  return change;
}

/** The streak as it stands now (it lapses after a day without three breaks). */
export function currentStreak(s: BreakSettings, now: Date): number {
  const today = dayKey(now);
  const yesterday = dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1));
  return s.countedDay === today || s.countedDay === yesterday ? s.streak : 0;
}

export const BREAK_TEXT: Record<BreakKind, { title: string; body: string }> = {
  eyes: { title: "Rest your eyes", body: "Look at something about 20 feet (6 m) away for 20 seconds." },
  stretch: {
    title: "Time to stretch",
    body: "Stand up, roll your shoulders, stretch your back for a minute.",
  },
  water: { title: "Drink some water", body: "A glass of water keeps the headaches away." },
};
