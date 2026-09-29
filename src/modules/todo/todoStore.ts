import { create } from "zustand";

import type { Task } from "../../core/generated/Task";
import type { TodoSnapshot } from "../../core/generated/TodoSnapshot";

interface TodoState {
  snapshot: TodoSnapshot | null;
  set: (snapshot: TodoSnapshot) => void;
}

export const useTodo = create<TodoState>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
}));

/** Running timers, soonest first. */
export function runningTimers(tasks: Task[]): Task[] {
  return tasks
    .filter((t) => t.timerEndsAt)
    .sort((a, b) => new Date(a.timerEndsAt!).getTime() - new Date(b.timerEndsAt!).getTime());
}
