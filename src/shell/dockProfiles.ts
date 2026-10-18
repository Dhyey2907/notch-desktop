// Dock profiles: a different dock for work, games or study. The active one can follow the app in
// front (and the time of day). Pure helpers here, tested; the island switches profiles.
import type { DockProfile } from "../core/generated/DockProfile";
import type { DockSettings } from "../core/generated/DockSettings";

/** The dock to show: the active profile's order and hidden icons, or the main dock's. */
export function activeDock(dock: DockSettings): {
  order: string[];
  hidden: string[];
  profile: DockProfile | null;
} {
  const profile = dock.profiles.find((p) => p.id === dock.active) ?? null;
  return profile
    ? { order: profile.order, hidden: profile.hidden, profile }
    : { order: dock.order, hidden: dock.hidden, profile: null };
}

/** Whether a minute of the day falls in a profile's hours (which may wrap past midnight). */
export function inHours(profile: DockProfile, minute: number): boolean {
  const { fromMin, toMin } = profile;
  if (fromMin === undefined || toMin === undefined || fromMin === toMin) return false;
  return fromMin < toMin ? minute >= fromMin && minute < toMin : minute >= fromMin || minute < toMin;
}

/**
 * Which profile fits now: one listing the app in front wins, then one whose hours it is,
 * otherwise the main dock ("").
 */
export function pickProfile(
  profiles: readonly DockProfile[],
  frontExe: string | null,
  minute: number,
): string {
  const exe = (frontExe ?? "").toLowerCase().replace(/\.exe$/, "");
  if (exe) {
    const byApp = profiles.find((p) => p.apps.some((a) => a.toLowerCase().replace(/\.exe$/, "") === exe));
    if (byApp) return byApp.id;
  }
  return profiles.find((p) => inHours(p, minute))?.id ?? "";
}

/** Starter profiles: copies of the current dock to make your own. */
export function starterProfiles(order: string[], hidden: string[]): DockProfile[] {
  return [
    {
      id: "work",
      name: "Work",
      order: [...order],
      hidden: [...hidden],
      apps: [],
      fromMin: 9 * 60,
      toMin: 18 * 60,
    },
    {
      id: "gaming",
      name: "Gaming",
      order: [...order],
      hidden: [...hidden],
      apps: ["steam", "epicgameslauncher"],
    },
    { id: "study", name: "Study", order: [...order], hidden: [...hidden], apps: [] },
  ];
}
