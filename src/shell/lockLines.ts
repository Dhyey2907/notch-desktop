// The few lines drawn onto the lock-screen picture (Settings → Look → Lock-screen info).
// Pure, so it's tested.
import type { CalendarEvent } from "../core/generated/CalendarEvent";
import type { MediaState } from "../core/generated/MediaState";
import type { Weather } from "../core/generated/Weather";

export interface LockInputs {
  next: CalendarEvent | null;
  weather: Weather | null;
  media: MediaState | null | undefined;
  unread: number | null;
}

export function lockLines({ next, weather, media, unread }: LockInputs, now: number): string[] {
  const lines: string[] = [];
  if (next && !next.allDay) {
    const at = new Date(next.start);
    const mins = Math.round((at.getTime() - now) / 60_000);
    const when =
      mins <= 0
        ? "now"
        : mins < 60
          ? `in ${mins} min`
          : at.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
    lines.push(`Next: ${next.title || "Meeting"} · ${when}`);
  }
  if (weather?.temperature != null) {
    lines.push(`${Math.round(weather.temperature)}°${weather.condition ? ` · ${weather.condition}` : ""}`);
  }
  if (media && media.status === "playing" && !media.mock && media.title) {
    lines.push(`♪ ${media.title}${media.artist ? ` — ${media.artist}` : ""}`);
  }
  if (unread && unread > 0) lines.push(`${unread} unread ${unread === 1 ? "mail" : "mails"}`);
  return lines;
}
