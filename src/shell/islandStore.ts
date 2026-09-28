// Island presentation state machine (architecture.md §4). Window resizing is sequenced
// around animations so content never clips:
//   1. before animating, grow the window to fit both the current and target states;
//   2. animate the island;
//   3. when the animation settles, shrink the window to the target state.
import { create } from "zustand";

import { sameSize, unionSize, windowSizeFor, type IslandMode, type Size } from "./geometry";

export type ResizeWindow = (size: Size) => Promise<void>;

export interface IslandState {
  mode: IslandMode;
  /** Ask for a mode change; resolves once the island has started animating toward it. */
  request: (mode: IslandMode) => Promise<void>;
  toggleExpanded: () => Promise<void>;
  /** Call when the island's animation settles, to trim the window to the current mode. */
  settle: () => Promise<void>;
}

export function createIslandStore(resizeWindow: ResizeWindow, initialMode: IslandMode = "collapsed") {
  let windowSize: Size | null = null;
  let latestRequest = 0;

  const resizeTo = async (size: Size) => {
    if (sameSize(windowSize, size)) return;
    windowSize = size;
    try {
      await resizeWindow(size);
    } catch (error) {
      windowSize = null; // unknown; force the next resize to go through
      console.error("failed to resize island window", error);
    }
  };

  return create<IslandState>((set, get) => ({
    mode: initialMode,

    request: async (mode) => {
      const current = get().mode;
      if (mode === current) return;
      const requestId = ++latestRequest;
      await resizeTo(unionSize(windowSizeFor(current), windowSizeFor(mode)));
      // A newer request arrived while we were resizing; let it win.
      if (requestId !== latestRequest) return;
      set({ mode });
    },

    toggleExpanded: () => get().request(get().mode === "expanded" ? "collapsed" : "expanded"),

    settle: () => resizeTo(windowSizeFor(get().mode)),
  }));
}
