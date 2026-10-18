import { create } from "zustand";

import type { Wallpaper } from "../../core/generated/Wallpaper";
import type { WallpaperState } from "../../core/generated/WallpaperState";

export type WallpaperTab = "recent" | "saved" | "folder";

export const ALL_SCREENS = "all";

interface WallpaperStore {
  state: WallpaperState | null;
  set: (state: WallpaperState) => void;
  /** None picked yet: Recent when there is any, else Saved. */
  tab: WallpaperTab | null;
  setTab: (tab: WallpaperTab) => void;
}

export const useWallpaper = create<WallpaperStore>((set) => ({
  state: null,
  set: (state) => set({ state }),
  tab: null,
  setTab: (tab) => set({ tab }),
}));

export function shownTab(tab: WallpaperTab | null, state: WallpaperState | null): WallpaperTab {
  if (tab) return tab;
  return state && state.recent.length > 0 ? "recent" : "saved";
}

export function itemsFor(state: WallpaperState | null, tab: WallpaperTab): Wallpaper[] {
  if (!state) return [];
  return tab === "recent" ? state.recent : tab === "saved" ? state.saved : state.folder;
}

/** "Screen 1 · Screen 2 · All" (only the screens when there's just one). */
export function screenOptions(state: WallpaperState | null): { value: string; label: string }[] {
  const screens = state?.screens ?? [];
  const options = screens.map((s) => ({ value: s.id, label: s.label }));
  return screens.length > 1 ? [...options, { value: ALL_SCREENS, label: "All" }] : options;
}

/** The target a click uses: the chosen one if it's still connected, else every screen. */
export function effectiveTarget(state: WallpaperState | null): string {
  const target = state?.target ?? ALL_SCREENS;
  return state?.screens.some((s) => s.id === target) ? target : ALL_SCREENS;
}

/** On the chosen screen(s) now (for a check mark). */
export function isCurrent(state: WallpaperState | null, path: string): boolean {
  if (!state) return false;
  const target = effectiveTarget(state);
  const screens = target === ALL_SCREENS ? state.screens : state.screens.filter((s) => s.id === target);
  return screens.length > 0 && screens.every((s) => s.current?.toLowerCase() === path.toLowerCase());
}

/** Where a click put it, for the status line. */
export function targetLabel(state: WallpaperState | null, target: string): string {
  if (target === ALL_SCREENS) return (state?.screens.length ?? 0) > 1 ? "every screen" : "your desktop";
  return state?.screens.find((s) => s.id === target)?.label ?? "your desktop";
}

const PICTURE = /\.(jpe?g|png|webp|bmp)$/i;

export function picturesOnly(paths: string[]): string[] {
  return paths.filter((p) => PICTURE.test(p));
}
