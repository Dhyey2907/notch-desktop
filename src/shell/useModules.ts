import { useEffect } from "react";

import { useActivities } from "../activity";
import { eventBus } from "../core/eventBus";
import { MODULES } from "../modules/registry";
import type { ModuleContext } from "../modules/types";
import { createModuleStore } from "./moduleStore";

/** App-wide module navigation store. */
export const useModuleNav = createModuleStore(MODULES.map((m) => m.id));

/** Run each module's `init` once and dispose on unmount. One failing module can't stop the rest. */
export function useModuleLifecycle() {
  useEffect(() => {
    const { push, dismiss } = useActivities.getState();
    const ctx: ModuleContext = { events: eventBus, activities: { push, dismiss } };
    const disposers: (() => void)[] = [];
    for (const module of MODULES) {
      try {
        const dispose = module.init?.(ctx);
        if (dispose) disposers.push(dispose);
      } catch (error) {
        console.error(`module ${module.id} failed to initialize`, error);
      }
    }
    return () => disposers.forEach((dispose) => dispose());
  }, []);
}
