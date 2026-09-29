import { create } from "zustand";

import type { AgentSession } from "../../core/generated/AgentSession";
import type { ClaudeStatus } from "../../core/generated/ClaudeStatus";

export interface ClaudeStoreState {
  status: ClaudeStatus | null;
  /** Bumped whenever new transcript data is indexed, so usage views re-query. */
  usageVersion: number;
  setStatus: (status: ClaudeStatus) => void;
  bumpUsage: () => void;
}

export const useClaude = create<ClaudeStoreState>((set) => ({
  status: null,
  usageVersion: 0,
  setStatus: (status) => set({ status }),
  bumpUsage: () => set((s) => ({ usageVersion: s.usageVersion + 1 })),
}));

/** The session to feature: waiting beats working beats idle; most recent first within each. */
export function primarySession(sessions: AgentSession[]): AgentSession | null {
  const rank = { waiting: 0, working: 1, idle: 2 } as const;
  return [...sessions].sort((a, b) => rank[a.state] - rank[b.state])[0] ?? null;
}

/** 48,812,345 → "48.8M"; 12,400 → "12.4K"; 950 → "950". */
export function formatTokens(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1).replace(/\.0$/, "")}B`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1).replace(/\.0$/, "")}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1).replace(/\.0$/, "")}K`;
  return String(n);
}

/** "claude-opus-5-5" → "Opus 5.5"; "claude-sonnet-5" → "Sonnet 5"; unknown ids pass through. */
export function prettyModel(id: string): string {
  const match = /^claude-([a-z]+)-(\d+)(?:-(\d+))?(?:-\d{8})?$/.exec(id);
  if (!match) return id;
  const [, family, major, minor] = match;
  return `${family.charAt(0).toUpperCase()}${family.slice(1)} ${major}${minor ? `.${minor}` : ""}`;
}

/** 15 → "3 PM", 0 → "12 AM". */
export function formatHour(hour: number): string {
  const suffix = hour < 12 ? "AM" : "PM";
  return `${hour % 12 === 0 ? 12 : hour % 12} ${suffix}`;
}

/** "↑ 23% vs previous 7 days", "↓ 5% …", or null when there's nothing to compare. */
export function comparisonText(current: number, previous: number | undefined, days: number): string | null {
  if (previous === undefined) return null;
  if (previous === 0) return current > 0 ? `New activity vs the previous ${days} days` : null;
  const change = Math.round(((current - previous) / previous) * 100);
  if (change === 0) return `Same as the previous ${days} days`;
  return `${change > 0 ? "↑" : "↓"} ${Math.abs(change)}% tokens vs the previous ${days} days`;
}

/** "five_hour" → "5-hour", "seven_day" → "weekly". */
export function limitLabel(type: string): string {
  return (
    { five_hour: "5-hour", seven_day: "weekly", seven_day_opus: "weekly Opus" }[type] ??
    type.replace(/_/g, " ")
  );
}
