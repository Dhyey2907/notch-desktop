// "When should I remind you?" — quick choices and how a reminder's time reads.

export interface RemindChoice {
  label: string;
  /** Epoch milliseconds. */
  at: number;
}

/** In 1 hour, this evening at 6 (while it's still ahead), tomorrow at 9, next Monday at 9. */
export function remindChoices(now: Date): RemindChoice[] {
  const at = (days: number, hour: number) =>
    new Date(now.getFullYear(), now.getMonth(), now.getDate() + days, hour, 0, 0, 0).getTime();
  const choices: RemindChoice[] = [{ label: "In 1 hour", at: now.getTime() + 3600_000 }];
  if (now.getHours() < 17) choices.push({ label: "This evening", at: at(0, 18) });
  choices.push({ label: "Tomorrow morning", at: at(1, 9) });
  const toMonday = (8 - now.getDay()) % 7 || 7;
  choices.push({ label: "Next Monday", at: at(toMonday, 9) });
  return choices;
}

/** "6:00 PM", "Tomorrow 9:00 AM", "Mon 12 Oct, 9:00 AM". */
export function reminderLabel(at: number, now: Date): string {
  const when = new Date(at);
  const time = when.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  const day = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((day(when) - day(now)) / 86_400_000);
  if (days === 0) return time;
  if (days === 1) return `Tomorrow ${time}`;
  return `${when.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" })}, ${time}`;
}

/** The value for an <input type="datetime-local"> (local time, minutes). */
export function toLocalInput(at: number): string {
  const d = new Date(at);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}
