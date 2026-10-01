import { create } from "zustand";

import type { DevServer } from "../../core/generated/DevServer";

export type DevTab = "servers" | "time" | "run" | "widgets";

export const useDevTools = create<{
  servers: DevServer[];
  tab: DevTab;
  setTab: (tab: DevTab) => void;
}>((set) => ({
  servers: [],
  tab: "servers",
  setTab: (tab) => set({ tab }),
}));

/** 95 → "1h 35m". */
export function hoursMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m ? `${h}h ${m}m` : `${h}h`;
}
