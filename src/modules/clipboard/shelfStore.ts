import { create } from "zustand";

import type { BridgeState } from "../../core/generated/BridgeState";
import type { DownloadProgress } from "../../core/generated/DownloadProgress";
import type { RecorderState } from "../../core/generated/RecorderState";
import type { ShelfSnapshot } from "../../core/generated/ShelfSnapshot";

export type ClipboardTab = "copied" | "files" | "recent" | "read";

interface ShelfState {
  snapshot: ShelfSnapshot | null;
  set: (snapshot: ShelfSnapshot) => void;
  tab: ClipboardTab;
  setTab: (tab: ClipboardTab) => void;
  /** Files are being dragged over this island. */
  dropHover: boolean;
  setDropHover: (on: boolean) => void;
  /** The phone bridge, while it runs. */
  bridge: BridgeState | null;
  setBridge: (bridge: BridgeState | null) => void;
  /** Downloads in progress (from any browser). */
  downloads: DownloadProgress[];
  /** Recording what the PC plays. */
  recorder: RecorderState;
}

export const useShelf = create<ShelfState>((set) => ({
  snapshot: null,
  set: (snapshot) => set({ snapshot }),
  tab: "copied",
  setTab: (tab) => set({ tab }),
  dropHover: false,
  setDropHover: (dropHover) => set({ dropHover }),
  bridge: null,
  setBridge: (bridge) => set({ bridge }),
  downloads: [],
  recorder: { recording: false },
}));

const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** 2_400_000 -> "2.3 MB". */
export function formatSize(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${unit === 0 || value >= 10 ? Math.round(value) : value.toFixed(1)} ${UNITS[unit]}`;
}
