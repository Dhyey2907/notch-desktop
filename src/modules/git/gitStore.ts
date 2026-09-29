import { create } from "zustand";

import type { ChangeCounts } from "../../core/generated/ChangeCounts";
import type { DayCount } from "../../core/generated/DayCount";
import type { GitSnapshot } from "../../core/generated/GitSnapshot";
import type { HeatmapCell } from "../../design/components";
import { heatmapLevel } from "../../design/heatmap";

export interface GitStoreState {
  snapshot: GitSnapshot | null;
  set: (snapshot: GitSnapshot) => void;
}

export const useGit = create<GitStoreState>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
}));

export function totalChanges(c: ChangeCounts): number {
  return c.staged + c.modified + c.untracked + c.conflicted;
}

/** "12 changed files", "1 changed file", or "No changes". */
export function changeSummary(c: ChangeCounts): string {
  const total = totalChanges(c);
  if (total === 0) return "No changes";
  return `${total} changed ${total === 1 ? "file" : "files"}`;
}

/** Files `git commit --all` would include (tracked changes, staged or not). */
export function trackedChanges(c: ChangeCounts): number {
  return c.staged + c.modified;
}

export function activityCells(days: DayCount[]): HeatmapCell[] {
  const max = Math.max(0, ...days.map((d) => d.count));
  return days.map((d) => ({ date: d.date, value: d.count, level: heatmapLevel(d.count, max) }));
}

/** "just now", "5m ago", "3h ago", "2d ago", "4mo ago", "15y ago". */
export function timeAgo(iso: string, now: number): string {
  const minutes = Math.floor((now - Date.parse(iso)) / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 60) return `${days}d ago`;
  if (days < 365) return `${Math.floor(days / 30)}mo ago`;
  return `${Math.floor(days / 365)}y ago`;
}
