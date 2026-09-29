import { Timer } from "lucide-react";

import { formatCountdown } from "./parse";
import { runningTimers, useTodo } from "./todoStore";
import { useSecondClock } from "./useSecondClock";
import "./todo.css";

/** Bubble: a timer mark while any timer runs. */
export function TodoCompact() {
  const running = useTodo((s) => runningTimers(s.snapshot?.tasks ?? []).length > 0);
  if (!running) return null;
  return (
    <span className="todo-compact" role="img" aria-label="A timer is running">
      <Timer size={12} aria-hidden />
    </span>
  );
}

/** Hover: the soonest timer counting down, or how many tasks are open. */
export function TodoPeek() {
  const tasks = useTodo((s) => s.snapshot?.tasks ?? []);
  const next = runningTimers(tasks)[0];
  const now = useSecondClock(Boolean(next));
  const open = tasks.filter((t) => !t.done).length;

  return (
    <div className="module-peek">
      <span className="module-peek__title">
        {next ? `⏱ ${next.title}` : open ? `${open} ${open === 1 ? "task" : "tasks"} to do` : "Nothing to do"}
      </span>
      <span className="module-peek__meta">
        {next
          ? `${formatCountdown(new Date(next.timerEndsAt!).getTime() - now)} left`
          : "Click to add a task or note"}
      </span>
    </div>
  );
}
