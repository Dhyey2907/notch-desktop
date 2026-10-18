// A routine made from a habit Overline noticed (apps that start together at about the same
// time). Pure, so it's tested.
import type { HabitSuggestion } from "../../core/generated/HabitSuggestion";
import type { Routine } from "../../core/generated/Routine";

/** "spotify" → "Spotify", "msedge" → "Msedge". */
export function label(exe: string): string {
  return exe.charAt(0).toUpperCase() + exe.slice(1);
}

/** "9:33". */
export function clock(min: number): string {
  return `${Math.floor(min / 60)}:${String(min % 60).padStart(2, "0")}`;
}

export function habitRoutine(s: HabitSuggestion, id: string, workDays: number[]): Routine {
  return {
    id,
    name: `Start: ${s.exes.slice(0, 3).map(label).join(", ")}${s.exes.length > 3 ? "…" : ""}`,
    steps: s.paths.map((path) => ({ kind: "app" as const, path })),
    atMin: s.atMin,
    days: workDays.length > 0 ? workDays : [1, 2, 3, 4, 5],
  };
}

export function habitText(s: HabitSuggestion): string {
  const names = s.exes.map(label);
  const list = names.length > 1 ? `${names.slice(0, -1).join(", ")} and ${names.at(-1)}` : names[0];
  return `You usually open ${list} around ${clock(s.atMin + 2)}. Open them all at ${clock(s.atMin)} on work days?`;
}
