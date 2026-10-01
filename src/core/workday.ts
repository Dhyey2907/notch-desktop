// Workday progress and the weekend countdown (Settings → Home → Workday).
import type { DaySettings } from "./generated/DaySettings";

export interface Workday {
  /** "before" work, "during", "after" work, or "off" (not a work day). */
  phase: "before" | "during" | "after" | "off";
  /** 0–1 through the workday (during only). */
  fraction: number;
  /** Minutes until the workday ends (during), or starts (before). */
  minutes: number;
}

/** Monday = 1 … Sunday = 7. */
const isoDay = (d: Date) => ((d.getDay() + 6) % 7) + 1;

export function workday(day: DaySettings, now: Date): Workday {
  if (!day.workDays.includes(isoDay(now))) return { phase: "off", fraction: 0, minutes: 0 };
  const min = now.getHours() * 60 + now.getMinutes();
  if (min < day.startMin) return { phase: "before", fraction: 0, minutes: day.startMin - min };
  if (min >= day.endMin) return { phase: "after", fraction: 1, minutes: 0 };
  return {
    phase: "during",
    fraction: (min - day.startMin) / (day.endMin - day.startMin),
    minutes: day.endMin - min,
  };
}

/**
 * Minutes until the weekend starts: the end of the last work day before a day off.
 * `null` on a day off (it's already the weekend) or when every day is a work day.
 */
export function weekendIn(day: DaySettings, now: Date): number | null {
  if (day.workDays.length === 0 || day.workDays.length === 7) return null;
  if (!day.workDays.includes(isoDay(now))) return null;
  const end = new Date(now);
  // Walk forward while tomorrow is also a work day.
  for (let i = 0; i < 7; i++) {
    const next = new Date(end);
    next.setDate(end.getDate() + 1);
    if (!day.workDays.includes(isoDay(next))) break;
    end.setDate(end.getDate() + 1);
  }
  end.setHours(Math.floor(day.endMin / 60), day.endMin % 60, 0, 0);
  const minutes = Math.round((end.getTime() - now.getTime()) / 60_000);
  return minutes > 0 ? minutes : null;
}

/** 3h 20m / 2d 4h. */
export function span(minutes: number): string {
  if (minutes >= 24 * 60) {
    const d = Math.floor(minutes / (24 * 60));
    const h = Math.floor((minutes % (24 * 60)) / 60);
    return h ? `${d}d ${h}h` : `${d}d`;
  }
  if (minutes >= 60) {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    return m ? `${h}h ${m}m` : `${h}h`;
  }
  return `${minutes}m`;
}
