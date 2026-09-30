import { create } from "zustand";

import type { Clip } from "../../core/generated/Clip";
import type { ClipboardSnapshot } from "../../core/generated/ClipboardSnapshot";

export const useClipboard = create<{
  snapshot: ClipboardSnapshot | null;
  set: (snapshot: ClipboardSnapshot) => void;
}>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
}));

/** The most recent copy (pinned clips aren't "latest"). */
export function latestClip(snapshot: ClipboardSnapshot | null): Clip | null {
  return snapshot?.recent[0] ?? null;
}
