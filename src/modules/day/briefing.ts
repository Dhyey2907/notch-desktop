// The morning briefing (Settings → Your day): the first time you're at the PC after 5 am, one
// card with the weather, your meetings, open tasks and unread mail. Built here, on this PC;
// nothing is sent anywhere. Pure, so it's tested.
import type { CalendarEvent } from "../../core/generated/CalendarEvent";
import type { Weather } from "../../core/generated/Weather";

/** The briefing shows from this hour on. */
export const MORNING_FROM = 5;

/** Is it time for today's briefing? (After 5 am, not shown today, and you're here.) */
export function briefingDue(
  now: Date,
  shownDay: string | null,
  today: string,
  activeRecently: boolean,
): boolean {
  return now.getHours() >= MORNING_FROM && shownDay !== today && activeRecently;
}

export interface BriefingInputs {
  weather: Weather | null;
  /** Today's timed events (any order). */
  events: CalendarEvent[];
  openTasks: number;
  unread: number | null;
}

function time(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

export function briefingLines({ weather, events, openTasks, unread }: BriefingInputs, now: Date): string[] {
  const lines: string[] = [];
  if (weather?.temperature != null) {
    lines.push(
      `${Math.round(weather.temperature)}°${weather.condition ? ` ${weather.condition.toLowerCase()}` : ""}`,
    );
  }
  const timed = events.filter((e) => !e.allDay).sort((a, b) => Date.parse(a.start) - Date.parse(b.start));
  const upcoming = timed.filter((e) => Date.parse(e.start) >= now.getTime());
  if (upcoming.length > 0) {
    const first = upcoming[0];
    const more = upcoming.length - 1;
    lines.push(`${first.title || "Meeting"} at ${time(first.start)}${more > 0 ? ` (+${more} more)` : ""}`);
  } else if (timed.length === 0) {
    lines.push("No meetings today");
  }
  if (openTasks > 0) lines.push(`${openTasks} open ${openTasks === 1 ? "task" : "tasks"}`);
  if (unread && unread > 0) lines.push(`${unread} unread ${unread === 1 ? "mail" : "mails"}`);
  return lines;
}

/** "Good morning" until noon (the briefing can be late if you start late). */
export function greeting(now: Date): string {
  const h = now.getHours();
  return h < 12 ? "Good morning" : h < 17 ? "Good afternoon" : "Good evening";
}
