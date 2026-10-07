import { create } from "zustand";

import type { ReadLaterSnapshot } from "../../core/generated/ReadLaterSnapshot";

interface ReadLaterState {
  snapshot: ReadLaterSnapshot | null;
  set: (snapshot: ReadLaterSnapshot) => void;
}

export const useReadLater = create<ReadLaterState>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
}));

/** Minutes to read at about 230 words a minute ("1 min" at least). */
export function readingMinutes(words: number | undefined): number | null {
  if (!words) return null;
  return Math.max(1, Math.round(words / 230));
}

/** The first web link in some text, if any. */
export function findLink(text: string): string | null {
  const match = /https?:\/\/[^\s<>"')]+/i.exec(text);
  return match ? match[0].replace(/[.,;:!?]+$/, "") : null;
}
