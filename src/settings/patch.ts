// Small helpers for settings pages: change part of one settings group, or reset groups.
import type { Settings } from "../core/generated/Settings";
import { ipc } from "../core/ipc";
import { useSettings } from ".";

type Group = {
  [K in keyof Settings]: Settings[K] extends object ? (Settings[K] extends unknown[] ? never : K) : never;
}[keyof Settings];

/** Change some fields of one settings group, e.g. `patch("appearance", { theme: "glass" })`. */
export function patch<K extends Group>(group: K, change: Partial<Settings[K]>) {
  void useSettings.getState().update((s) => ({ ...s, [group]: { ...s[group], ...change } }));
}

/** "Reset this page": put these groups back to their defaults. */
export async function resetGroups(groups: Group[]) {
  const defaults = await ipc.settingsDefaults();
  await useSettings.getState().update((s) => {
    const next = { ...s };
    for (const g of groups) (next as Record<string, unknown>)[g] = defaults[g];
    return next;
  });
}
