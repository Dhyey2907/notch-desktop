// Typed wrappers over Tauri commands. Components call these, never `invoke` directly,
// so every backend call is discoverable in one place (plan.md Rule 4).
import { invoke } from "@tauri-apps/api/core";

import type { AppInfo } from "./generated/AppInfo";
import type { DevTestActivity } from "./generated/DevTestActivity";

/** Shape of `AppError` as serialized by the backend. */
export interface IpcError {
  kind: string;
  message: string;
}

export function isIpcError(value: unknown): value is IpcError {
  return typeof value === "object" && value !== null && "kind" in value && "message" in value;
}

export const ipc = {
  appInfo: () => invoke<AppInfo>("app_info"),
  diagnosticsPing: (nonce: string) => invoke<void>("diagnostics_ping", { nonce }),
  /** Resize the island window (logical px); the backend keeps it pinned top-center. */
  /** Debug builds only: fire a fake activity through the real backend event pipeline. */
  devEmitTestActivity: (activity: DevTestActivity, delayMs?: number) =>
    invoke<void>("dev_emit_test_activity", { activity, delayMs }),
  setWindowSize: (width: number, height: number) => invoke<void>("shell_set_window_size", { width, height }),
};
