// Island presentation state machine (architecture.md §4). Window resizing is sequenced
// around animations so content never clips:
//   1. before animating, grow the window to fit both the current and target states;
//   2. animate the island;
//   3. when the animation settles, shrink the window to the target state.
import { create } from "zustand";

import { ISLAND_SIZE, sameSize, unionSize, windowSizeFor, type IslandMode, type Size } from "./geometry";

export type ResizeWindow = (size: Size) => Promise<void>;

export interface IslandState {
  mode: IslandMode;
  /** Current island size (logical px); defaults per mode, activities may override. */
  size: Size;
  /** Ask for a mode change; resolves once the island has started animating toward it. */
  request: (mode: IslandMode, size?: Size) => Promise<void>;
  toggleExpanded: () => Promise<void>;
  /** Call when the island's animation settles, to trim the window to the current state. */
  settle: () => Promise<void>;
  /** Resize the window again even if the state didn't change (the notch size setting did). */
  refit: () => Promise<void>;
  /** Width of the resting bubble: wider when it has more to show (music, downloads, pet…). */
  bubbleWidth: number;
  setBubbleWidth: (width: number) => Promise<void>;
  /** Natural width of the module dock; the expanded window is at least this wide. */
  dockWidth: number;
  setDockWidth: (width: number) => Promise<void>;
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
    size: ISLAND_SIZE[initialMode],

    bubbleWidth: ISLAND_SIZE.collapsed.width,
    dockWidth: 0,

    setDockWidth: async (width) => {
      if (width === get().dockWidth) return;
      set({ dockWidth: width });
      if (get().mode === "expanded") await resizeTo(windowSizeFor("expanded", get().size, width));
    },

    setBubbleWidth: async (width) => {
      if (width === get().bubbleWidth) return;
      set({ bubbleWidth: width });
      if (get().mode === "collapsed") await get().request("collapsed");
    },

    request: async (mode, requested) => {
      const size =
        requested ??
        (mode === "collapsed" ? { ...ISLAND_SIZE.collapsed, width: get().bubbleWidth } : ISLAND_SIZE[mode]);
      const current = get();
      if (mode === current.mode && sameSize(current.size, size)) return;
      const requestId = ++latestRequest;
      await resizeTo(
        unionSize(
          windowSizeFor(current.mode, current.size, current.dockWidth),
          windowSizeFor(mode, size, current.dockWidth),
        ),
      );
      // A newer request arrived while we were resizing; let it win.
      if (requestId !== latestRequest) return;
      set({ mode, size });
    },

    toggleExpanded: () => get().request(get().mode === "expanded" ? "collapsed" : "expanded"),

    settle: () => resizeTo(windowSizeFor(get().mode, get().size, get().dockWidth)),

    refit: () => {
      windowSize = null;
      return resizeTo(windowSizeFor(get().mode, get().size, get().dockWidth));
    },
  }));
}
