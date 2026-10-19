// Panel layouts (Edit layout in the dock): each panel's sections in the order you chose, minus
// the ones you hid, and how big the panel opens. Pure helpers here (tested); panels call
// `useArranged`, the editor calls `savePanel`.
import { useMemo } from "react";
import { create } from "zustand";

import type { PanelLayout } from "../core/generated/PanelLayout";
import type { PanelSize } from "../core/generated/PanelSize";
import { useSettings } from "../settings";
import type { Size } from "./geometry";

export interface LayoutItem {
  id: string;
  label: string;
}

/** A list a panel lets you arrange: its sections, or a row inside it (Control's toggles). */
export interface LayoutGroup {
  /** Settings key: the module id for its sections, "<module>.<list>" for a list inside it. */
  id: string;
  label: string;
  items: readonly LayoutItem[];
  /** Keep at least one shown (an empty list would leave a blank panel). */
  keepOne?: boolean;
}

export const PANEL_SIZES: Record<PanelSize, Size> = {
  normal: { width: 540, height: 300 },
  tall: { width: 540, height: 400 },
  wide: { width: 680, height: 300 },
  large: { width: 680, height: 400 },
};

export const PANEL_SIZE_LABELS: Record<PanelSize, string> = {
  normal: "Normal",
  tall: "Tall",
  wide: "Wide",
  large: "Large",
};

/** All items in the chosen order; ones never arranged keep their built-in place after those. */
export function ordered<T extends { id: string }>(items: readonly T[], layout?: PanelLayout): T[] {
  const order = layout?.order ?? [];
  const rank = (id: string, i: number) => {
    const at = order.indexOf(id);
    return at === -1 ? order.length + i : at;
  };
  return items
    .map((item, i) => ({ item, r: rank(item.id, i) }))
    .sort((a, b) => a.r - b.r)
    .map((x) => x.item);
}

/** What a panel shows: the chosen order without the hidden ones. */
export function arrange<T extends { id: string }>(items: readonly T[], layout?: PanelLayout): T[] {
  const hidden = layout?.hidden ?? [];
  return ordered(items, layout).filter((item) => !hidden.includes(item.id));
}

/** Move `id` one step up (-1) or down (+1) in the full order. */
export function nudge(ids: readonly string[], id: string, by: -1 | 1): string[] {
  const i = ids.indexOf(id);
  const j = i + by;
  if (i === -1 || j < 0 || j >= ids.length) return [...ids];
  const next = [...ids];
  [next[i], next[j]] = [next[j], next[i]];
  return next;
}

export function usePanelLayout(id: string): PanelLayout | undefined {
  return useSettings((s) => s.settings?.layout?.panels[id]);
}

/** A panel's items as you arranged them. `items` should be stable (module-level or memoized). */
export function useArranged<T extends { id: string }>(id: string, items: readonly T[]): T[] {
  const layout = usePanelLayout(id);
  return useMemo(() => arrange(items, layout), [items, layout]);
}

/** Save part of a panel's layout. */
export function savePanel(id: string, patch: Partial<PanelLayout>) {
  return useSettings.getState().update((s) => {
    const panels = { ...s.layout.panels };
    const current: PanelLayout = panels[id] ?? { order: [], hidden: [], size: "normal" };
    panels[id] = { ...current, ...patch };
    return { ...s, layout: { ...s.layout, panels } };
  });
}

/** Forget a panel's arrangement (and its lists'). */
export function resetPanel(moduleId: string) {
  return useSettings.getState().update((s) => {
    const panels = Object.fromEntries(
      Object.entries(s.layout.panels).filter(([id]) => id !== moduleId && !id.startsWith(`${moduleId}.`)),
    );
    return { ...s, layout: { ...s.layout, panels } };
  });
}

/** Edit layout: shows the editor in place of the open panel. */
export const useLayoutEdit = create<{ editing: boolean; set: (editing: boolean) => void }>((set) => ({
  editing: false,
  set: (editing) => set({ editing }),
}));
