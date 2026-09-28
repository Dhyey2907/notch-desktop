import { eventBus } from "../core/eventBus";
import { ipc } from "../core/ipc";
import { createSettingsStore } from "./settingsStore";

/** App-wide settings store for the current window. */
export const useSettings = createSettingsStore(
  { get: ipc.settingsGet, update: ipc.settingsUpdate },
  eventBus,
);
