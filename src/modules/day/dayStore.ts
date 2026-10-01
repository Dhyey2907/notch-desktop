import { create } from "zustand";

import type { DayState } from "../../core/generated/DayState";

export type DayTab = "plan" | "due";

export const useDay = create<{ state: DayState | null; tab: DayTab }>(() => ({ state: null, tab: "plan" }));

/** "in 5 h", "tomorrow", "Fri", "2 h ago". */
export function dueIn(due: string, now: number): string {
  const ms = new Date(due).getTime() - now;
  const hours = Math.round(ms / 3_600_000);
  if (ms < 0) return hours <= -1 ? `${-hours} h ago` : "just now";
  if (hours < 1) return `in ${Math.max(1, Math.round(ms / 60_000))} min`;
  if (hours < 20) return `in ${hours} h`;
  const days = Math.round(ms / 86_400_000);
  if (days <= 1) return "tomorrow";
  if (days < 7) return new Date(due).toLocaleDateString(undefined, { weekday: "short" });
  return new Date(due).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

/** A plan line split into its time ("09:30–10:30") and what to do. */
export function planLine(line: string): { time?: string; text: string } {
  const m = /^\s*[-•*]?\s*(\d{1,2}[:.]\d{2}(?:\s*[–-]\s*\d{1,2}[:.]\d{2})?)\s*[:·-]?\s+(.*)$/.exec(line);
  return m ? { time: m[1].replace(/\s/g, ""), text: m[2] } : { text: line.replace(/^\s*[-•*]\s*/, "") };
}
