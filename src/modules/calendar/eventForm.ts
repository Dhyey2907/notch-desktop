// The new-event form's values → the start and end Google gets.
import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import type { NewEvent } from "../../core/generated/NewEvent";

export const DURATIONS = [15, 30, 60, 90, 120];

/** "2026-10-07" for a date input (local). */
export function dateInput(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** The next half hour from now, as "HH:MM". */
export function nextHalfHour(now: Date): string {
  const d = new Date(now);
  d.setMinutes(d.getMinutes() < 30 ? 30 : 60, 0, 0);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export function buildEvent(
  title: string,
  date: string,
  time: string,
  minutes: number,
  allDay: boolean,
  location: string,
): NewEvent | null {
  const [y, m, d] = date.split("-").map(Number);
  if (!y || !m || !d) return null;
  let start: Date;
  let end: Date;
  if (allDay) {
    start = new Date(y, m - 1, d);
    end = new Date(y, m - 1, d + 1);
  } else {
    const clock = /^(\d{1,2}):(\d{2})$/.exec(time);
    if (!clock) return null;
    const [h, min] = [Number(clock[1]), Number(clock[2])];
    start = new Date(y, m - 1, d, h, min);
    end = new Date(start.getTime() + minutes * 60_000);
  }
  return {
    title: title.trim(),
    start: start.toISOString(),
    end: end.toISOString(),
    allDay,
    location: location.trim() || undefined,
  };
}

/** Invites from others still waiting for your answer, soonest first. */
export function waitingInvites(events: CalendarEvent[], now: number): CalendarEvent[] {
  return events
    .filter((e) => e.myResponse === "needsAction" && Date.parse(e.end) > now)
    .sort((a, b) => Date.parse(a.start) - Date.parse(b.start));
}
