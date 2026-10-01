import { create } from "zustand";

import type { ControlSnapshot } from "../../core/generated/ControlSnapshot";

export const useControl = create<{ snapshot: ControlSnapshot | null; set: (s: ControlSnapshot) => void }>(
  (set) => ({
    snapshot: null,
    set: (snapshot) => set({ snapshot }),
  }),
);
