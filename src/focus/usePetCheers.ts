import { useEffect } from "react";

import { eventBus } from "../core/eventBus";
import { cheerPet } from "./petCheer";

/** Listens (on every island, always mounted) for things that make the pet cheer. */
export function usePetCheers() {
  useEffect(() => {
    let doneCount: number | null = null;
    const offFocus = eventBus.on("FocusDone", () => cheerPet());
    // A task ticked off anywhere makes it happy too.
    const offTodo = eventBus.on("TodoChanged", ({ event }) => {
      const done = event.payload.tasks.filter((t) => t.done).length;
      if (doneCount !== null && done > doneCount) cheerPet();
      doneCount = done;
    });
    return () => {
      offFocus();
      offTodo();
    };
  }, []);
}
