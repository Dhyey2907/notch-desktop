import { create } from "zustand";

/** Text to start the palette with (the quick-note shortcut types "note "), and when it was set. */
export const usePalette = create<{ prefill: string; at: number }>(() => ({ prefill: "", at: 0 }));
