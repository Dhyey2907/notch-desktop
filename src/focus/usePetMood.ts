import { useEffect, useState } from "react";

import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { focusActive, useFocus } from "./focusStore";
import { petMood, type PetMood } from "./petMood";

const CHEER_MS = 6000;
const IDLE_CHECK_MS = 60_000;

/** The pet's mood and whether a 3+ session streak earned it a star today. */
export function usePetMood(): { mood: PetMood; streak: boolean } {
  const state = useFocus((s) => s.state);
  const [cheeringUntil, setCheeringUntil] = useState(0);
  const [idleMs, setIdleMs] = useState(0);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const cheer = () => {
      const until = Date.now() + CHEER_MS;
      setCheeringUntil(until);
      setNow(Date.now());
      window.setTimeout(() => setNow(Date.now()), CHEER_MS + 50);
    };
    let doneCount: number | null = null;
    const offFocus = eventBus.on("FocusDone", cheer);
    // A task ticked off anywhere makes it happy too.
    const offTodo = eventBus.on("TodoChanged", ({ event }) => {
      const done = event.payload.tasks.filter((t) => t.done).length;
      if (doneCount !== null && done > doneCount) cheer();
      doneCount = done;
    });
    const check = () =>
      ipc.systemIdleMs().then(
        (ms) => {
          setIdleMs(ms);
          setNow(Date.now());
        },
        () => {},
      );
    void check();
    const id = window.setInterval(() => void check(), IDLE_CHECK_MS);
    return () => {
      offFocus();
      offTodo();
      window.clearInterval(id);
    };
  }, []);

  return {
    mood: petMood({ cheeringUntil, focusing: focusActive(state, now), idleMs, now }),
    streak: (state?.doneToday ?? 0) >= 3,
  };
}
