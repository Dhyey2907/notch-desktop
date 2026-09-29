// Plan usage limits (5-hour and weekly) as reported by Claude Code's status line.
import type { Tone } from "../../design/components";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** "Resets in 1 hr 29 min" within a day; otherwise the weekday and time, "Resets Wed 5:30 PM". */
export function resetText(resetsAt: string, now: number): string {
  const at = new Date(resetsAt).getTime();
  const left = at - now;
  if (left <= 0) return "Resetting now";
  if (left < HOUR) return `Resets in ${Math.max(1, Math.ceil(left / MINUTE))} min`;
  if (left < DAY) {
    const hours = Math.floor(left / HOUR);
    const minutes = Math.floor((left % HOUR) / MINUTE);
    return `Resets in ${hours} hr${minutes ? ` ${minutes} min` : ""}`;
  }
  const when = new Date(at).toLocaleString(undefined, {
    weekday: "short",
    hour: "numeric",
    minute: "2-digit",
  });
  return `Resets ${when}`;
}

/** "Updated just now", "Updated 3 min ago", "Updated 2 hr ago", "Updated Sep 28". */
export function updatedText(updatedAt: string, now: number): string {
  const ago = now - new Date(updatedAt).getTime();
  if (ago < MINUTE) return "Updated just now";
  if (ago < HOUR) return `Updated ${Math.floor(ago / MINUTE)} min ago`;
  if (ago < DAY) return `Updated ${Math.floor(ago / HOUR)} hr ago`;
  return `Updated ${new Date(updatedAt).toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;
}

/** Blue while comfortable, amber when getting close, red near the cap. */
export function limitTone(usedPercent: number): Tone {
  if (usedPercent >= 90) return "danger";
  if (usedPercent >= 70) return "warning";
  return "info";
}
