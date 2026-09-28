// Frontend copy of the backend-owned settings. Every window loads once, then follows
// `SettingsChanged` events so the island and the settings window never disagree.
import { create } from "zustand";

import type { EventBus } from "../core/eventBus";
import type { ModuleSettings } from "../core/generated/ModuleSettings";
import type { Settings } from "../core/generated/Settings";

export const DEFAULT_MODULE_SETTINGS: ModuleSettings = {
  enabled: true,
  mayPeek: true,
  initFailures: 0,
  autoDisabled: false,
};

export function moduleSettings(settings: Settings | null, moduleId: string): ModuleSettings {
  return { ...DEFAULT_MODULE_SETTINGS, ...settings?.modules[moduleId] };
}

export interface SettingsBackend {
  get: () => Promise<Settings>;
  update: (settings: Settings) => Promise<Settings>;
}

export interface SettingsState {
  settings: Settings | null;
  load: () => Promise<void>;
  /** Apply a change on top of the latest settings and persist it. */
  update: (change: (draft: Settings) => Settings) => Promise<void>;
  setModule: (moduleId: string, patch: Partial<ModuleSettings>) => Promise<void>;
}

export function createSettingsStore(backend: SettingsBackend, events: Pick<EventBus, "on">) {
  const store = create<SettingsState>((set, get) => ({
    settings: null,

    load: async () => set({ settings: await backend.get() }),

    update: async (change) => {
      const current = get().settings ?? (await backend.get());
      set({ settings: await backend.update(change(current)) });
    },

    setModule: (moduleId, patch) =>
      get().update((s) => ({
        ...s,
        modules: { ...s.modules, [moduleId]: { ...moduleSettings(s, moduleId), ...patch } },
      })),
  }));

  events.on("SettingsChanged", ({ event }) => store.setState({ settings: event.payload }));
  return store;
}
