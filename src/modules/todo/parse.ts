// Quick add: "Write report 25m" → task "Write report" with a 25-minute timer.

const MAX_MINUTES = 24 * 60;

/** A trailing duration like "25m", "25 min", "1h", "1h 30m", "1 hr 30 mins", "90 minutes". */
const TRAILING_DURATION =
  /\s+(?:(\d{1,2})\s*(?:h|hr|hrs|hour|hours))?\s*(?:(\d{1,4})\s*(?:m|min|mins|minute|minutes))?\s*$/i;

export interface QuickAdd {
  title: string;
  minutes?: number;
}

export function parseQuickAdd(input: string): QuickAdd {
  const text = input.trim();
  const match = TRAILING_DURATION.exec(text);
  if (match && (match[1] || match[2])) {
    const minutes = Number(match[1] ?? 0) * 60 + Number(match[2] ?? 0);
    const title = text.slice(0, match.index).trim();
    if (title && minutes >= 1 && minutes <= MAX_MINUTES) return { title, minutes };
  }
  return { title: text };
}

/** "25 min", "1 h", "1 h 30 min". */
export function formatMinutes(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m} min`;
  return m === 0 ? `${h} h` : `${h} h ${m} min`;
}

/** Countdown text: "4:05", "12:34", "1:02:03"; never negative. */
export function formatCountdown(ms: number): string {
  const total = Math.max(0, Math.ceil(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}
