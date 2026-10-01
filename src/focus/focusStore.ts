// Focus session state shared by the island (holding cards), the bubble (ring, pet) and the
// To-do module's Focus tab.
import { create } from "zustand";

import type { ActivityInput } from "../activity/activityStore";
import type { FocusState } from "../core/generated/FocusState";

/** Held cards kept for the summary (newest last). */
const MAX_HELD = 50;

interface FocusStore {
  state: FocusState | null;
  set: (state: FocusState) => void;
  /** Cards held back during the current session. */
  held: ActivityInput[];
  takeHeld: () => ActivityInput[];
}

export const useFocus = create<FocusStore>((set, get) => ({
  state: null,
  set: (state) => set({ state }),
  held: [],
  takeHeld: () => {
    const held = get().held;
    set({ held: [] });
    return held;
  },
}));

export function focusActive(state: FocusState | null, now: number): boolean {
  return Boolean(state?.endsAt && Date.parse(state.endsAt) > now);
}

/** Ordinary notifications wait; approvals, timers, reminders, HUDs and critical cards don't. */
export function shouldHold(input: ActivityInput): boolean {
  return !input.hud && (input.priority === "normal" || input.priority === "low");
}

/** `hold` for the activity store: during focus, keep non-urgent cards for later. */
export function holdDuringFocus(input: ActivityInput): boolean {
  const store = useFocus.getState();
  if (!focusActive(store.state, Date.now()) || !shouldHold(input)) return false;
  useFocus.setState({ held: [...store.held, input].slice(-MAX_HELD) });
  return true;
}

/** "3 notifications · Phone 2, Git 1". */
export function heldSummary(held: ActivityInput[], names: (moduleId: string) => string): string {
  const counts = new Map<string, number>();
  for (const h of held) counts.set(names(h.moduleId), (counts.get(names(h.moduleId)) ?? 0) + 1);
  const parts = [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([name, n]) => `${name} ${n}`);
  return `${held.length} ${held.length === 1 ? "notification" : "notifications"} · ${parts.join(", ")}`;
}

/** 0–1 of the session done. */
export function focusProgress(state: FocusState | null, now: number): number {
  if (!state?.endsAt || !state.startedAt) return 0;
  const start = Date.parse(state.startedAt);
  const end = Date.parse(state.endsAt);
  return Math.min(1, Math.max(0, (now - start) / Math.max(1, end - start)));
}
