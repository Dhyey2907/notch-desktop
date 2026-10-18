// Rearranging the dock inside the notch: long-press an icon, then drag icons sideways. Pure
// helpers here (tested); the Dock draws it.
import { create } from "zustand";

/** Hold an icon this long to start rearranging. */
export const LONG_PRESS_MS = 500;
/** Moving further than this before the long press ends cancels it (it was a drag or a swipe). */
export const PRESS_SLOP_PX = 8;

/** Move `id` to `to` (an index in the list without it). */
export function moveId(ids: readonly string[], id: string, to: number): string[] {
  const rest = ids.filter((x) => x !== id);
  const at = Math.max(0, Math.min(rest.length, to));
  return [...rest.slice(0, at), id, ...rest.slice(at)];
}

/** Where a dragged icon belongs, from the pointer's x and the other icons' centres (in order). */
export function slotFor(x: number, centers: readonly number[]): number {
  let slot = 0;
  for (const c of centers) if (x > c) slot += 1;
  return slot;
}

interface DockEdit {
  editing: boolean;
  /** The order while editing (saved on Done). */
  draft: string[] | null;
  dragging: string | null;
  start: (ids: readonly string[], dragging?: string | null) => void;
  setDraft: (ids: string[]) => void;
  setDragging: (id: string | null) => void;
  stop: () => void;
}

export const useDockEdit = create<DockEdit>((set) => ({
  editing: false,
  draft: null,
  dragging: null,
  start: (ids, dragging = null) => set({ editing: true, draft: [...ids], dragging }),
  setDraft: (draft) => set({ draft }),
  setDragging: (dragging) => set({ dragging }),
  stop: () => set({ editing: false, draft: null, dragging: null }),
}));
