// Island-wide focus behaviour: keep the focus state current, and when a session ends show what
// was held back (and celebrate a session that ran its full length).
import { useEffect } from "react";

import { useActivities } from "../activity";
import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { findModule } from "../modules/registry";
import { focusActive, heldSummary, useFocus } from "./focusStore";

/** Cards shown again from "Show" (the most recent ones). */
const SHOW_HELD = 5;

export function showHeldSummary(title = "While you focused") {
  const held = useFocus.getState().takeHeld();
  if (held.length === 0) return;
  useActivities.getState().push({
    moduleId: "todo",
    priority: "normal",
    tone: "info",
    dedupKey: "focus-held",
    countRepeats: false,
    timeoutMs: 15_000,
    title,
    body: heldSummary(held, (id) => findModule(id)?.name ?? id),
    actions: [
      {
        id: "show",
        label: held.length > SHOW_HELD ? `Show latest ${SHOW_HELD}` : "Show",
        variant: "primary",
        run: () => {
          for (const card of held.slice(-SHOW_HELD)) useActivities.getState().push(card);
        },
      },
    ],
  });
}

export function useFocusSession() {
  useEffect(() => {
    let wasActive = false;
    const apply = (state: Parameters<ReturnType<typeof useFocus.getState>["set"]>[0]) => {
      useFocus.getState().set(state);
      const active = focusActive(state, Date.now());
      if (wasActive && !active) showHeldSummary();
      wasActive = active;
    };
    const offChanged = eventBus.on("FocusChanged", ({ event }) => apply(event.payload));
    const offDone = eventBus.on("FocusDone", () => {
      const done = useFocus.getState().state?.doneToday ?? 0;
      useActivities.getState().push({
        moduleId: "todo",
        priority: "high",
        tone: "success",
        dedupKey: "focus-done",
        countRepeats: false,
        timeoutMs: 8000,
        title: "Focus session done",
        body: done > 1 ? `${done} sessions today — nice streak` : "Nice work. Take a short break.",
      });
    });
    ipc.focusState().then(apply, () => {});
    return () => {
      offChanged();
      offDone();
    };
  }, []);
}
