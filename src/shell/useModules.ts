import { useEffect, useMemo } from "react";

import { useActivities } from "../activity";
import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { MODULES } from "../modules/registry";
import type { ModuleContext, NotchModule } from "../modules/types";
import { useSettings } from "../settings";
import { moduleSettings } from "../settings/settingsStore";
import { createModuleStore } from "./moduleStore";

/** App-wide module navigation store. */
export const useModuleNav = createModuleStore(MODULES.map((m) => m.id));

export function isModuleEnabled(module: NotchModule, settings: Parameters<typeof moduleSettings>[0]) {
  return module.alwaysOn || moduleSettings(settings, module.id).enabled;
}

/** Modules the user has enabled, in dock order. */
export function useEnabledModules(): readonly NotchModule[] {
  const settings = useSettings((s) => s.settings);
  return useMemo(() => MODULES.filter((m) => isModuleEnabled(m, settings)), [settings]);
}

/** A context scoped to one module, so the shell can apply that module's preferences. */
function contextFor(module: NotchModule): ModuleContext {
  const { push, dismiss, update } = useActivities.getState();
  return {
    events: eventBus,
    activities: {
      push: (input) => {
        // "May interrupt" off: drop non-critical activities. Critical ones (approvals)
        // always get through — silencing them would hide a waiting agent.
        const { mayPeek } = moduleSettings(useSettings.getState().settings, module.id);
        if (!mayPeek && input.priority !== "critical") return "";
        return push(input);
      },
      dismiss,
      update,
    },
  };
}

/**
 * Run each enabled module's `init` and dispose it when disabled or on unmount. Outcomes are
 * reported so a module that keeps failing is disabled instead of breaking every startup.
 */
export function useModuleLifecycle() {
  const loaded = useSettings((s) => s.settings !== null);
  const enabled = useEnabledModules();
  const enabledKey = enabled.map((m) => m.id).join(",");

  useEffect(() => {
    useModuleNav.getState().setIds(enabled.map((m) => m.id));
  }, [enabled]);

  useEffect(() => {
    if (!loaded) return;
    const disposers: (() => void)[] = [];
    for (const module of enabled) {
      if (!module.init) continue;
      const hadFailures = moduleSettings(useSettings.getState().settings, module.id).initFailures > 0;
      try {
        const dispose = module.init(contextFor(module));
        if (dispose) disposers.push(dispose);
        // Only write when there is a failure streak to clear.
        if (hadFailures) void ipc.settingsRecordModuleInit(module.id, true);
      } catch (error) {
        console.error(`module ${module.id} failed to initialize`, error);
        void ipc.settingsRecordModuleInit(module.id, false);
      }
    }
    return () => disposers.forEach((dispose) => dispose());
    // Re-run only when the set of enabled modules changes, not on every settings write.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loaded, enabledKey]);
}
