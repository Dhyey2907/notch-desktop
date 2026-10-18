import { useEffect, useMemo } from "react";

import { useActivities } from "../activity";
import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { MODULES } from "../modules/registry";
import type { ModuleContext, NotchModule } from "../modules/types";
import { useSettings } from "../settings";
import { moduleSettings } from "../settings/settingsStore";
import { activeDock } from "./dockProfiles";
import { createModuleStore } from "./moduleStore";

/** App-wide module navigation store. */
export const useModuleNav = createModuleStore(MODULES.map((m) => m.id));

export function isModuleEnabled(module: NotchModule, settings: Parameters<typeof moduleSettings>[0]) {
  return module.alwaysOn || moduleSettings(settings, module.id).enabled;
}

/** Order modules as the user arranged them; ones not arranged yet keep their built-in order. */
export function dockOrder<T extends { id: string }>(modules: readonly T[], order: readonly string[]): T[] {
  const rank = (id: string) => {
    const i = order.indexOf(id);
    return i === -1 ? order.length : i;
  };
  return modules
    .map((m, i) => ({ m, i }))
    .sort((a, b) => rank(a.m.id) - rank(b.m.id) || a.i - b.i)
    .map(({ m }) => m);
}

/** Modules the user has enabled, in dock order (the active dock profile's). */
export function useEnabledModules(): readonly NotchModule[] {
  const settings = useSettings((s) => s.settings);
  return useMemo(
    () =>
      dockOrder(
        MODULES.filter((m) => isModuleEnabled(m, settings)),
        settings ? activeDock(settings.dock).order : [],
      ),
    [settings],
  );
}

/** Save a new dock order: to the profile in use, or the main dock. */
export async function saveDockOrder(order: string[]) {
  await useSettings.getState().update((s) => {
    const active = s.dock.profiles.find((p) => p.id === s.dock.active);
    if (!active) return { ...s, dock: { ...s.dock, order } };
    return {
      ...s,
      dock: { ...s.dock, profiles: s.dock.profiles.map((p) => (p.id === active.id ? { ...p, order } : p)) },
    };
  });
}

/** Enabled modules shown in the dock (some can be hidden from it but stay on). */
export function useDockModules(): readonly NotchModule[] {
  const enabled = useEnabledModules();
  const hidden = useSettings((s) => (s.settings ? activeDock(s.settings.dock).hidden : undefined));
  return useMemo(() => {
    const dockable = enabled.filter((m) => !m.dockless);
    const shown = dockable.filter((m) => !hidden?.includes(m.id));
    return shown.length > 0 ? shown : dockable;
  }, [enabled, hidden]);
}

/** Settings → Dock: "Open the notch to". Unset means wherever it was last. */
export function openFirstModule() {
  const first = useSettings.getState().settings?.dock.first;
  if (first) useModuleNav.getState().setActive(first);
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
  const docked = useDockModules();
  const enabledKey = enabled.map((m) => m.id).join(",");

  useEffect(() => {
    const ids = docked.map((m) => m.id);
    useModuleNav.getState().setIds(
      ids,
      enabled.map((m) => m.id).filter((id) => !ids.includes(id)),
    );
  }, [enabled, docked]);

  // Start on the chosen module once settings load.
  useEffect(() => {
    if (loaded) openFirstModule();
  }, [loaded]);

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
