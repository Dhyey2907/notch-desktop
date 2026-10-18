import { create } from "zustand";

import type { PriceItem } from "../../core/generated/PriceItem";

export const usePrices = create<{ items: PriceItem[]; set: (items: PriceItem[]) => void }>((set) => ({
  items: [],
  set: (items) => set({ items }),
}));
