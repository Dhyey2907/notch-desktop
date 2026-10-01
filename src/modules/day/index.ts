// Day: an AI plan for the rest of today (from your tasks, meetings and deadlines; made by
// Claude Code on workday mornings or on request) and Google Classroom work that's due soon.
import { Sunrise } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { dueIn, useDay } from "./dayStore";

function init({ events, activities }: ModuleContext) {
  const offChanged = events.on("DayChanged", ({ event }) => useDay.setState({ state: event.payload }));
  const offReady = events.on("DayPlanReady", ({ event }) =>
    activities.push({
      moduleId: "day",
      priority: "normal",
      tone: "info",
      dedupKey: "day-plan",
      countRepeats: false,
      timeoutMs: 12_000,
      title: "Your plan for today is ready",
      body: event.payload.first_line,
    }),
  );
  const offDue = events.on("AssignmentDue", ({ event }) => {
    const a = event.payload;
    activities.push({
      moduleId: "day",
      priority: "high",
      tone: "warning",
      dedupKey: `due-${a.id}`,
      countRepeats: false,
      title: `Due ${dueIn(a.due, Date.now())}: ${a.title}`,
      body: a.course,
      actions: a.link
        ? [
            {
              id: "open",
              label: "Open in Classroom",
              variant: "primary",
              run: () => ipc.openExternal(a.link!),
            },
          ]
        : undefined,
    });
  });
  ipc.dayState().then(
    (state) => useDay.setState({ state }),
    () => {},
  );
  return () => {
    offChanged();
    offReady();
    offDue();
  };
}

export const dayModule: NotchModule = {
  id: "day",
  name: "Day",
  icon: Sunrise,
  order: 1,
  init,
  Expanded: lazy(() => import("./DayExpanded")),
};
