// The pet's day-to-day: how full it is (it gets a little hungry while you work, and treats fill
// it up), your play streak, and what it has unlocked with its level. Pure, so it's tested.

/** Fullness lost per hour you're at the PC (never while you're away or asleep). */
export const HUNGER_PER_HOUR = 8;
export const TREAT_FILLS = 12;
export const FETCH_FILLS = 4;
/** Only the first few fetches a day give xp. */
export const FETCH_XP_PER_DAY = 10;

export type Hunger = "full" | "fine" | "peckish" | "sulky";

export function hungerOf(fullness: number): Hunger {
  if (fullness >= 60) return "full";
  if (fullness >= 30) return "fine";
  if (fullness >= 10) return "peckish";
  return "sulky";
}

/** After `activeMinutes` at the PC. */
export function digest(fullness: number, activeMinutes: number): number {
  return Math.max(0, Math.round(fullness - (activeMinutes * HUNGER_PER_HOUR) / 60));
}

export function feed(fullness: number, amount = TREAT_FILLS): number {
  return Math.min(100, Math.round(fullness + amount));
}

/** YYYY-MM-DD, local. */
export function dayKey(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

/** You played with it today: keep (or start) the streak. */
export function played(
  streakDays: number,
  lastPlayDay: string,
  now: Date,
): { streakDays: number; lastPlayDay: string } {
  const today = dayKey(now);
  if (lastPlayDay === today) return { streakDays: Math.max(1, streakDays), lastPlayDay };
  const yesterday = dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1));
  return { streakDays: lastPlayDay === yesterday ? streakDays + 1 : 1, lastPlayDay: today };
}

/** The streak as it stands today (a missed day ends it). */
export function currentStreak(streakDays: number, lastPlayDay: string, now: Date): number {
  const today = dayKey(now);
  const yesterday = dayKey(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1));
  return lastPlayDay === today || lastPlayDay === yesterday ? streakDays : 0;
}

/** A collar tag after a week of play. */
export const COLLAR_STREAK = 7;

export interface Unlock {
  id: string;
  label: string;
  level: number;
}

/** Hats, in the order they unlock (ids match PET_HATS in Rust). */
export const HATS: Unlock[] = [
  { id: "cap", label: "Cap", level: 2 },
  { id: "bow", label: "Bow", level: 3 },
  { id: "party", label: "Party hat", level: 4 },
  { id: "crown", label: "Crown", level: 6 },
  { id: "wizard", label: "Wizard hat", level: 9 },
];

/** Tricks it does now and then, and on a double-click. */
export const TRICKS: Unlock[] = [
  { id: "backflip", label: "Backflip", level: 5 },
  { id: "juggle", label: "Juggling", level: 8 },
];

export function unlocked(list: Unlock[], level: number): Unlock[] {
  return list.filter((u) => u.level <= level);
}

/** Fetch counts for today, and whether this one still gives xp. */
export function fetched(
  fetchesToday: number,
  fetchDay: string,
  now: Date,
): { fetchesToday: number; fetchDay: string; xp: number } {
  const today = dayKey(now);
  const count = fetchDay === today ? fetchesToday + 1 : 1;
  return { fetchesToday: count, fetchDay: today, xp: count <= FETCH_XP_PER_DAY ? 1 : 0 };
}
