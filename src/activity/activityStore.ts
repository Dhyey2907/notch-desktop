// Live Activity engine (plan.md Phase 5): decides what claims the island.
//
// Rules:
//  - Order: priority (critical > high > normal > low), then most recently updated first.
//  - Non-persistent activities expire after a per-priority timeout; critical ones persist
//    until acted on. One timer is armed for the next expiry — no polling (plan.md Rule 7).
//  - An activity with the same `dedupKey` (from the same module) replaces the old one in
//    place, keeping its id and counting repeats, instead of stacking.
//  - Everything not in front is summarized as a count, never flooded onto the desktop.
import type { LucideIcon } from "lucide-react";
import { create } from "zustand";

import type { ActivityPriority } from "../core/generated/ActivityPriority";
import type { Tone } from "../design/components";

export type { ActivityPriority };

export const PRIORITY_RANK: Record<ActivityPriority, number> = { critical: 0, high: 1, normal: 2, low: 3 };

/** Default visible lifetime per priority; `null` means persistent until acted on. */
export const DEFAULT_TIMEOUT_MS: Record<ActivityPriority, number | null> = {
  critical: null,
  high: 8000,
  normal: 5000,
  low: 3000,
};

/** Upper bound on queued activities; the least important are dropped first. */
export const MAX_ACTIVITIES = 50;

export interface ActivityAction {
  id: string;
  /** Visible text, or the accessible name when `icon` is set. */
  label: string;
  /** Render as a compact icon button (e.g. media controls) instead of a text button. */
  icon?: LucideIcon;
  /** Shown but not clickable (e.g. the source doesn't support "next"). */
  disabled?: boolean;
  variant?: "primary" | "secondary" | "warning" | "danger";
  /** When set, the action needs a second confirming click showing this label. */
  confirmLabel?: string;
  run: () => void | Promise<void>;
  /** Keep the activity after the action runs (default: dismiss it). */
  keepOpen?: boolean;
}

export interface ActivityInput {
  moduleId: string;
  priority: ActivityPriority;
  title: string;
  body?: string;
  /** Short avatar text, usually a sender's initial. */
  avatar?: string;
  /** Small square image (e.g. album art) shown instead of the avatar. */
  imageUrl?: string;
  tone?: Tone;
  dedupKey?: string;
  /** Overrides the priority default: persistent activities never time out. */
  persistent?: boolean;
  timeoutMs?: number;
  actions?: ActivityAction[];
  /** Exact text to show verbatim in a monospace block (e.g. the command being approved). */
  code?: string;
  /** Show a "×N" count when replaced via `dedupKey` (default true). Off for e.g. track changes. */
  countRepeats?: boolean;
}

export interface Activity extends ActivityInput {
  id: string;
  createdAt: number;
  updatedAt: number;
  /** Epoch ms when it disappears, or `null` if persistent. */
  expiresAt: number | null;
  /** How many times it was replaced via its dedup key (1 = original only). */
  repeats: number;
}

export interface ActivityState {
  activities: Activity[];
  push: (input: ActivityInput) => string;
  dismiss: (id: string) => void;
  /** Refresh an activity's content in place (e.g. artwork arrived) without counting a repeat or restarting its timer. */
  update: (
    id: string,
    patch: Partial<Pick<ActivityInput, "title" | "body" | "avatar" | "imageUrl" | "tone" | "actions">>,
  ) => void;
  runAction: (id: string, actionId: string) => Promise<void>;
  /** Pause an activity's timeout, e.g. while the pointer rests on it. */
  hold: (id: string) => void;
  /** Resume after `hold`, restarting the full timeout. */
  release: (id: string) => void;
  clear: () => void;
}

export interface ActivityStoreOptions {
  now?: () => number;
  createId?: () => string;
}

function lifetime(input: ActivityInput): number | null {
  if (input.persistent) return null;
  return input.timeoutMs ?? DEFAULT_TIMEOUT_MS[input.priority];
}

/** Persistent activities (explicitly, or critical by default) need an explicit answer. */
export function isPersistent(input: ActivityInput): boolean {
  return lifetime(input) === null;
}

function compare(a: Activity, b: Activity): number {
  return PRIORITY_RANK[a.priority] - PRIORITY_RANK[b.priority] || b.updatedAt - a.updatedAt;
}

export function createActivityStore({
  now = Date.now,
  createId = () => crypto.randomUUID(),
}: ActivityStoreOptions = {}) {
  let timer: ReturnType<typeof setTimeout> | undefined;

  return create<ActivityState>((set, get) => {
    const commit = (activities: Activity[]) => {
      const sorted = [...activities].sort(compare).slice(0, MAX_ACTIVITIES);
      set({ activities: sorted });
      scheduleExpiry();
    };

    const scheduleExpiry = () => {
      clearTimeout(timer);
      const next = Math.min(...get().activities.map((a) => a.expiresAt ?? Infinity));
      if (next === Infinity) return;
      timer = setTimeout(expire, Math.max(0, next - now()));
    };

    const expire = () => {
      const t = now();
      commit(get().activities.filter((a) => a.expiresAt === null || a.expiresAt > t));
    };

    return {
      activities: [],

      push: (input) => {
        const t = now();
        const ms = lifetime(input);
        const expiresAt = ms === null ? null : t + ms;
        const existing = input.dedupKey
          ? get().activities.find((a) => a.dedupKey === input.dedupKey && a.moduleId === input.moduleId)
          : undefined;

        if (existing) {
          const replaced: Activity = {
            ...input,
            id: existing.id,
            createdAt: existing.createdAt,
            updatedAt: t,
            expiresAt,
            repeats: input.countRepeats === false ? 1 : existing.repeats + 1,
          };
          commit(get().activities.map((a) => (a.id === existing.id ? replaced : a)));
          return existing.id;
        }

        const id = createId();
        commit([...get().activities, { ...input, id, createdAt: t, updatedAt: t, expiresAt, repeats: 1 }]);
        return id;
      },

      dismiss: (id) => commit(get().activities.filter((a) => a.id !== id)),

      update: (id, patch) => commit(get().activities.map((a) => (a.id === id ? { ...a, ...patch } : a))),

      runAction: async (id, actionId) => {
        const action = get()
          .activities.find((a) => a.id === id)
          ?.actions?.find((x) => x.id === actionId);
        if (!action) return;
        try {
          await action.run();
        } catch (error) {
          // Keep the activity so the user can retry; the owning module reports details.
          console.error(`activity action ${actionId} failed`, error);
          return;
        }
        if (!action.keepOpen) get().dismiss(id);
      },

      hold: (id) => commit(get().activities.map((a) => (a.id === id ? { ...a, expiresAt: null } : a))),

      release: (id) => {
        const t = now();
        commit(
          get().activities.map((a) => {
            if (a.id !== id) return a;
            const ms = lifetime(a);
            return { ...a, expiresAt: ms === null ? null : t + ms };
          }),
        );
      },

      clear: () => commit([]),
    };
  });
}
