import { create } from "zustand";

import type { MeetingState } from "../../core/generated/MeetingState";

export const useMeeting = create<{ state: MeetingState; set: (state: MeetingState) => void }>((set) => ({
  state: { phase: "idle", ready: false },
  set: (state) => set({ state }),
}));

/** "1:05" / "1:02:05" since `startedAt` (ms). */
export function elapsed(startedAt: number, now: number): string {
  const total = Math.max(0, Math.floor((now - startedAt) / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}
