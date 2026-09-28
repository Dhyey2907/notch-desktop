// Which module the expanded island is showing (shell-owned navigation state, architecture.md §9).
import { create } from "zustand";

export interface ModuleNavState {
  ids: readonly string[];
  activeId: string;
  setActive: (id: string) => void;
  /** Move by `delta` positions, wrapping around (horizontal swipe / arrow keys). */
  step: (delta: number) => void;
  /** Replace the navigable modules (e.g. after enabling/disabling one in Settings). */
  setIds: (ids: readonly string[]) => void;
}

export function createModuleStore(ids: readonly string[]) {
  if (ids.length === 0) throw new Error("module store needs at least one module");
  return create<ModuleNavState>((set, get) => ({
    ids,
    activeId: ids[0],
    setActive: (id) => {
      if (get().ids.includes(id)) set({ activeId: id });
    },
    step: (delta) => {
      const { ids, activeId } = get();
      const index = ids.indexOf(activeId);
      set({ activeId: ids[(index + delta + ids.length * 10) % ids.length] });
    },
    setIds: (next) => {
      if (next.length === 0) return;
      const { activeId } = get();
      set({ ids: next, activeId: next.includes(activeId) ? activeId : next[0] });
    },
  }));
}
