import { Timer } from "lucide-react";

import { formatCountdown } from "./parse";
import { focusActive, focusProgress, useFocus } from "../../focus/focusStore";
import { runningTimers, useTodo } from "./todoStore";
import { useSecondClock } from "./useSecondClock";
import "./todo.css";

/** Bubble: a timer mark while any timer runs. */
export function TodoCompact() {
  const running = useTodo((s) => runningTimers(s.snapshot?.tasks ?? []).length > 0);
  const focus = useFocus((s) => s.state);
  const now = useSecondClock(Boolean(focus?.endsAt));
  if (focusActive(focus, now)) {
    const p = focusProgress(focus, now);
    return (
      <span className="todo-compact todo-compact--focus" role="img" aria-label="Focusing">
        <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden>
          <circle
            cx="6"
            cy="6"
            r="4.5"
            fill="none"
            stroke="currentColor"
            strokeOpacity="0.25"
            strokeWidth="1.6"
          />
          <circle
            cx="6"
            cy="6"
            r="4.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeDasharray={28.27}
            strokeDashoffset={28.27 * (1 - p)}
            transform="rotate(-90 6 6)"
          />
        </svg>
        {Math.max(1, Math.ceil((Date.parse(focus!.endsAt!) - now) / 60_000))}m
      </span>
    );
  }
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
