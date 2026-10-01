// To-do module: tasks with optional timers, and notes. When a timer finishes, the notch pops
// out with Done / +5 min / Dismiss until you acknowledge it (the backend rings, if asked).
import { ListTodo } from "lucide-react";
import { lazy } from "react";

import type { Task } from "../../core/generated/Task";
import type { TodoSnapshot } from "../../core/generated/TodoSnapshot";
import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { formatMinutes } from "./parse";
import { TodoCompact, TodoPeek } from "./TodoIndicators";
import { runningTimers, useTodo } from "./todoStore";

const SNOOZE_MINUTES = 5;

function init({ events, activities }: ModuleContext) {
  // Finished-timer cards on this screen, by task id.
  const shown = new Map<string, string>();

  const showTimerDone = (task: Task, late: boolean) => {
    const length = task.timerSecs ? formatMinutes(Math.round(task.timerSecs / 60)) : null;
    const id = activities.push({
      moduleId: "todo",
      priority: "high",
      persistent: true,
      tone: "info",
      dedupKey: `timer-${task.id}`,
      countRepeats: false,
      // No length means a reminder at a time (e.g. from a copied date).
      title: late
        ? `Missed while you were away: ${task.title}`
        : length
          ? `Time's up: ${task.title}`
          : `Reminder: ${task.title}`,
      body: length ? `${length} timer` : "Now",
      // Closing the card counts as dismissing the timer (stops the ring everywhere).
      onClose: () => void ipc.todoAcknowledge(task.id),
      actions: [
        { id: "dismiss", label: "Dismiss", variant: "secondary", run: () => ipc.todoAcknowledge(task.id) },
        {
          id: "snooze",
          label: `+${SNOOZE_MINUTES} min`,
          variant: "secondary",
          run: () => ipc.todoStartTimer(task.id, SNOOZE_MINUTES, task.ring),
        },
        { id: "done", label: "Done", variant: "primary", run: () => ipc.todoSetDone(task.id, true) },
      ],
    });
    shown.set(task.id, id);
  };

  const apply = (snapshot: TodoSnapshot) => {
    useTodo.getState().set(snapshot);
    // Acknowledged here or on another screen: close this screen's card too.
    for (const [taskId, activityId] of shown) {
      if (!snapshot.alerting.includes(taskId)) {
        activities.dismiss(activityId);
        shown.delete(taskId);
      }
    }
  };

  const offChanged = events.on("TodoChanged", ({ event }) => apply(event.payload));
  const offDone = events.on("TimerDone", ({ event }) =>
    showTimerDone(event.payload.task, event.payload.late),
  );
  ipc
    .todoSnapshot()
    .then((snapshot) => {
      if (!useTodo.getState().snapshot) apply(snapshot);
    })
    .catch((error) => console.error("failed to read to-do list", error));
  return () => {
    offChanged();
    offDone();
  };
}

export const todoModule: NotchModule = {
  id: "todo",
  name: "To-do",
  icon: ListTodo,
  order: 50,
  init,
  Compact: TodoCompact,
  Peek: TodoPeek,
  // A running timer makes hovering the notch preview it.
  usePeekClaim: () => useTodo((s) => runningTimers(s.snapshot?.tasks ?? []).length > 0),
  Expanded: lazy(() => import("./TodoExpanded")),
};
