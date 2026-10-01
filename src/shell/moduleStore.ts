// Which module the expanded island is showing (shell-owned navigation state, architecture.md §9).
import { create } from "zustand";

export interface ModuleNavState {
  ids: readonly string[];
  activeId: string;
  setActive: (id: string) => void;
  /** Move by `delta` positions, wrapping around (horizontal swipe / arrow keys). */
  step: (delta: number) => void;
  /** Replace the navigable modules (e.g. after enabling/disabling one in Settings).
   * `reachable` can be opened directly but is skipped by swipes and arrow keys. */
  setIds: (ids: readonly string[], reachable?: readonly string[]) => void;
  reachable: readonly string[];
}

export function createModuleStore(ids: readonly string[]) {
  if (ids.length === 0) throw new Error("module store needs at least one module");
  return create<ModuleNavState>((set, get) => ({
    ids,
    activeId: ids[0],
    reachable: [],
    setActive: (id) => {
      if (get().ids.includes(id) || get().reachable.includes(id)) set({ activeId: id });
    },
    step: (delta) => {
      const { ids, activeId } = get();
      const index = ids.indexOf(activeId);
      set({ activeId: ids[(index + delta + ids.length * 10) % ids.length] });
    },
    setIds: (next, reachable = []) => {
      if (next.length === 0) return;
      const { activeId } = get();
      const keep = next.includes(activeId) || reachable.includes(activeId);
      set({ ids: next, reachable, activeId: keep ? activeId : next[0] });
    },
  }));
}
