// The evening review: a card summing up your day — tasks done, focus sessions, coding time,
// screen time, meetings and your pet streak. Pure parts here, tested.

export interface ReviewInputs {
  tasksDone: number;
  focusSessions: number;
  /** Minutes per project, busiest first. */
  coding: { project: string; minutes: number }[];
  screenMinutes: number;
  meetings: number;
  petStreak: number;
}

export function hm(minutes: number): string {
  const h = Math.floor(minutes / 60);
  const m = Math.round(minutes % 60);
  return h > 0 ? (m > 0 ? `${h}h ${m}m` : `${h}h`) : `${m}m`;
}

/** The card's lines (nothing to say about an empty part of the day). */
export function reviewLines(i: ReviewInputs): string[] {
  const lines: string[] = [];
  if (i.tasksDone > 0) lines.push(`✓ ${i.tasksDone} ${i.tasksDone === 1 ? "task" : "tasks"} done`);
  if (i.focusSessions > 0)
    lines.push(`◎ ${i.focusSessions} focus ${i.focusSessions === 1 ? "session" : "sessions"}`);
  const coded = i.coding.reduce((a, p) => a + p.minutes, 0);
  if (coded > 0) {
    const top = i.coding
      .slice(0, 2)
      .map((p) => `${p.project} ${hm(p.minutes)}`)
      .join(", ");
    lines.push(`⌨ ${hm(coded)} coding (${top})`);
  }
  if (i.screenMinutes > 0) lines.push(`🖥 ${hm(i.screenMinutes)} at the screen`);
  if (i.meetings > 0) lines.push(`📅 ${i.meetings} ${i.meetings === 1 ? "meeting" : "meetings"}`);
  if (i.petStreak > 1) lines.push(`🐾 ${i.petStreak}-day play streak`);
  return lines;
}

/** Time for today's review: past the review minute and not shown yet today. */
export function reviewDue(now: Date, reviewAt: number, lastShownDay: string | null, dayKey: string): boolean {
  return now.getHours() * 60 + now.getMinutes() >= reviewAt && lastShownDay !== dayKey;
}
