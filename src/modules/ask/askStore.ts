import { create } from "zustand";

import type { AskSnapshot } from "../../core/generated/AskSnapshot";

export const useAsk = create<{
  snapshot: AskSnapshot | null;
  set: (snapshot: AskSnapshot) => void;
  /** More of an answer streamed in. */
  append: (id: string, text: string) => void;
}>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
  append: (id, text) =>
    set((s) =>
      s.snapshot
        ? {
            snapshot: {
              ...s.snapshot,
              exchanges: s.snapshot.exchanges.map((e) =>
                e.id === id ? { ...e, answer: e.answer + text } : e,
              ),
            },
          }
        : s,
    ),
}));
