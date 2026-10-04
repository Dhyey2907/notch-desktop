// Pure helpers for the Home dashboard.

/** "Good morning" / "Good afternoon" / "Good evening" / "Good night" for a local hour. */
export function greeting(hour: number): string {
  if (hour >= 5 && hour < 12) return "Good morning";
  if (hour >= 12 && hour < 17) return "Good afternoon";
  if (hour >= 17 && hour < 22) return "Good evening";
  return "Good night";
}

/** 378 -> "6h 18m"; 42 -> "42m"; 0 -> "0m". */
export function formatMinutes(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m}m`;
  return m === 0 ? `${h}h` : `${h}h ${m}m`;
}

/** "Saturday, September 26". */
/** "Today", "Yesterday", or "Thu 1 Oct" for a day `back` days before `now`. */
export function dayLabel(now: Date, back: number): string {
  if (back === 0) return "Today";
  if (back === 1) return "Yesterday";
  const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - back);
  return d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
}

/** YYYY-MM-DD (local) for a day `back` days before `now`. */
export function dayKeyBack(now: Date, back: number): string {
  const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - back);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

export function longDate(now: Date): string {
  return now.toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
}
