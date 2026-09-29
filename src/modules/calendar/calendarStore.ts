import { create } from "zustand";

import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import type { CalendarSnapshot } from "../../core/generated/CalendarSnapshot";

export interface CalendarStoreState {
  snapshot: CalendarSnapshot | null;
  set: (snapshot: CalendarSnapshot) => void;
}

export const useCalendar = create<CalendarStoreState>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
}));

/** Cached data older than this is labelled as possibly out of date. */
export const STALE_AFTER_MS = 30 * 60_000;

export function isStale(snapshot: CalendarSnapshot, now: number): boolean {
  return snapshot.lastSyncedAt != null && now - Date.parse(snapshot.lastSyncedAt) > STALE_AFTER_MS;
}

function startOfDay(t: Date): Date {
  return new Date(t.getFullYear(), t.getMonth(), t.getDate());
}

export function sameLocalDay(a: Date, b: Date): boolean {
  return startOfDay(a).getTime() === startOfDay(b).getTime();
}

/** Events touching the local day of `now`, all-day first, then by start. */
export function eventsOn(events: CalendarEvent[], day: Date): CalendarEvent[] {
  const from = startOfDay(day).getTime();
  const to = from + 86_400_000;
  return events
    .filter((e) => Date.parse(e.start) < to && Date.parse(e.end) > from)
    .sort((a, b) => Number(b.allDay) - Number(a.allDay) || Date.parse(a.start) - Date.parse(b.start));
}

/** Next timed event that hasn't started yet. */
export function nextEvent(events: CalendarEvent[], now: number): CalendarEvent | null {
  return (
    events
      .filter((e) => !e.allDay && Date.parse(e.start) > now)
      .sort((a, b) => Date.parse(a.start) - Date.parse(b.start))[0] ?? null
  );
}

export function isOngoing(event: CalendarEvent, now: number): boolean {
  return !event.allDay && Date.parse(event.start) <= now && now < Date.parse(event.end);
}

/** "in 12 min", "in 2 h 5 min", "now". */
export function relativeStart(event: CalendarEvent, now: number): string {
  const minutes = Math.ceil((Date.parse(event.start) - now) / 60_000);
  if (minutes <= 0) return "now";
  if (minutes < 60) return `in ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `in ${hours} h ${rest} min` : `in ${hours} h`;
}

/** Countdown body for a reminder: "Starts in 4:32", then "Started". */
export function countdownText(msUntilStart: number): string {
  if (msUntilStart <= 0) return "Started";
  const total = Math.ceil(msUntilStart / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return `Starts in ${minutes}:${seconds}`;
}

export function formatClock(iso: string): string {
  return new Date(iso).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

/**
 * Weeks of the month containing `day`, Monday first; `null` pads days outside the month.
 */
export function monthGrid(day: Date): (Date | null)[][] {
  const first = new Date(day.getFullYear(), day.getMonth(), 1);
  const daysInMonth = new Date(day.getFullYear(), day.getMonth() + 1, 0).getDate();
  const leading = (first.getDay() + 6) % 7; // Monday = 0
  const cells: (Date | null)[] = Array.from({ length: leading }, () => null);
  for (let d = 1; d <= daysInMonth; d++) cells.push(new Date(day.getFullYear(), day.getMonth(), d));
  while (cells.length % 7) cells.push(null);
  const weeks: (Date | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) weeks.push(cells.slice(i, i + 7));
  return weeks;
}
