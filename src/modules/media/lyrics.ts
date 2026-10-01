import { create } from "zustand";

import type { LyricLine } from "../../core/generated/LyricLine";
import type { Lyrics } from "../../core/generated/Lyrics";

export const useLyrics = create<{ lyrics: Lyrics | null; set: (l: Lyrics | null) => void }>((set) => ({
  lyrics: null,
  set: (lyrics) => set({ lyrics }),
}));

/** Index of the line being sung at `positionMs` (-1 before the first line). */
export function currentLineIndex(lines: LyricLine[], positionMs: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].atMs <= positionMs) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}
