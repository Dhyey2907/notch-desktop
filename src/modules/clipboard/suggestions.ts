// Cards for copied dates and addresses: "Remind me" (a To-do reminder that rings),
// "Add to Calendar" (Google Calendar's own prefilled event page — no sign-in needed), and
// "Open in Maps".
import type { ActivityInput } from "../../activity/activityStore";
import type { CopySuggestion } from "../../core/generated/CopySuggestion";

const pad = (n: number) => String(n).padStart(2, "0");

/** `2026-10-01T16:00:00` (local, no zone) -> Date in local time. */
export function localDate(iso: string): Date {
  const [d, t = "00:00:00"] = iso.split("T");
  const [y, mo, da] = d.split("-").map(Number);
  const [h, mi, s] = t.split(":").map(Number);
  return new Date(y, mo - 1, da, h, mi, s || 0);
}

/** "Thu 1 Oct, 4:00 PM" or "Thu 1 Oct" for all-day. */
export function whenLabel(at: Date, allDay: boolean): string {
  const day = at.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
  if (allDay) return day;
  return `${day}, ${at.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}`;
}

function stamp(d: Date, withTime: boolean): string {
  const date = `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`;
  return withTime ? `${date}T${pad(d.getHours())}${pad(d.getMinutes())}00` : date;
}

/** Google Calendar's "create event" page, prefilled (one hour long, or all day). */
export function calendarUrl(title: string, at: Date, allDay: boolean, timeZone: string): string {
  const end = allDay
    ? new Date(at.getFullYear(), at.getMonth(), at.getDate() + 1)
    : new Date(at.getTime() + 3_600_000);
  const params = new URLSearchParams({
    action: "TEMPLATE",
    text: title,
    dates: `${stamp(at, !allDay)}/${stamp(end, !allDay)}`,
    ctz: timeZone,
  });
  return `https://calendar.google.com/calendar/render?${params.toString()}`;
}

export function mapsUrl(address: string): string {
  return `https://www.google.com/maps/search/?api=1&query=${encodeURIComponent(address)}`;
}

export interface SuggestionRunners {
  remind: (title: string, atMs: number) => Promise<unknown>;
  open: (url: string) => Promise<void>;
  timeZone: string;
}

export function suggestionCard(s: CopySuggestion, run: SuggestionRunners): ActivityInput | null {
  const base = {
    moduleId: "clipboard",
    priority: "low" as const,
    tone: "info" as const,
    dedupKey: "copy-suggestion",
    countRepeats: false,
    timeoutMs: 8000,
  };
  if (s.remindAt) {
    const at = localDate(s.remindAt);
    return {
      ...base,
      title: whenLabel(at, s.allDay),
      body: s.text,
      actions: [
        {
          id: "calendar",
          label: "Add to Calendar",
          variant: "secondary",
          run: () => run.open(calendarUrl(s.text, at, s.allDay, run.timeZone)),
        },
        {
          id: "remind",
          label: "Remind me",
          variant: "primary",
          run: async () => {
            await run.remind(s.text, at.getTime());
          },
        },
      ],
    };
  }
  if (s.address) {
    const address = s.address;
    return {
      ...base,
      title: "Address copied",
      body: address,
      actions: [
        { id: "maps", label: "Open in Maps", variant: "primary", run: () => run.open(mapsUrl(address)) },
      ],
    };
  }
  return null;
}
