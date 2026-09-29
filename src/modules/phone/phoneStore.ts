import { create } from "zustand";

import type { PhoneSnapshot } from "../../core/generated/PhoneSnapshot";

export const usePhone = create<{ snapshot: PhoneSnapshot | null; set: (s: PhoneSnapshot) => void }>(
  (set) => ({
    snapshot: null,
    set: (snapshot) => set({ snapshot }),
  }),
);

/** "now", "5m", "2h", "Mon". */
export function ago(at: string, now: number): string {
  const minutes = Math.floor((now - new Date(at).getTime()) / 60_000);
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 24 * 60) return `${Math.floor(minutes / 60)}h`;
  return new Date(at).toLocaleDateString(undefined, { weekday: "short" });
}
