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
  /** On a side edge the resting bubble stands upright (its width and height swap). */
  vertical: boolean;
  setVertical: (vertical: boolean) => Promise<void>;
  /** Size of the open panel (Edit layout → size); resizes now if a panel is open. */
  expandedSize: Size;
  setExpandedSize: (size: Size) => Promise<void>;
  /**
   * Auto-hide has slid the resting bubble into the screen edge. Untucking grows the window
   * before the bubble slides out; tucking shrinks it once the slide settles (`settle`).
   * Any mode other than collapsed untucks.
   */
  tucked: boolean;
  setTucked: (tucked: boolean) => Promise<void>;
}

export function createIslandStore(resizeWindow: ResizeWindow, initialMode: IslandMode = "collapsed") {
  let windowSize: Size | null = null;
  let latestRequest = 0;
  let windowFor: (mode: IslandMode, size: Size) => Size = (mode, size) => windowSizeFor(mode, size);

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

  return create<IslandState>((set, get) => {
    windowFor = (mode, size) =>
      windowSizeFor(mode, size, get().dockWidth, {
        tucked: mode === "collapsed" && get().tucked,
        vertical: get().vertical,
      });
    return {
      mode: initialMode,
      size: ISLAND_SIZE[initialMode],

      bubbleWidth: ISLAND_SIZE.collapsed.width,
      dockWidth: 0,
      expandedSize: ISLAND_SIZE.expanded,
      vertical: false,
      tucked: false,

      setTucked: async (tucked) => {
        const { mode, size } = get();
        if (tucked === get().tucked || (tucked && mode !== "collapsed")) return;
        if (tucked) {
          set({ tucked });
          return;
        }
        // Room first, then slide out (the island animates once `tucked` flips).
        await resizeTo(windowSizeFor(mode, size, get().dockWidth, { vertical: get().vertical }));
        set({ tucked });
      },

      setVertical: async (vertical) => {
        if (vertical === get().vertical) return;
        set({ vertical });
        if (get().mode === "collapsed") await get().request("collapsed");
      },

      setExpandedSize: async (size) => {
        if (sameSize(get().expandedSize, size)) return;
        set({ expandedSize: size });
        if (get().mode === "expanded") await get().request("expanded");
      },

      setDockWidth: async (width) => {
        if (width === get().dockWidth) return;
        set({ dockWidth: width });
        if (get().mode === "expanded") await resizeTo(windowFor("expanded", get().size));
      },

      setBubbleWidth: async (width) => {
        if (width === get().bubbleWidth) return;
        set({ bubbleWidth: width });
        if (get().mode === "collapsed") await get().request("collapsed");
      },

      request: async (mode, requested) => {
        const wanted =
          requested ??
          (mode === "collapsed"
            ? { ...ISLAND_SIZE.collapsed, width: get().bubbleWidth }
            : mode === "expanded"
              ? get().expandedSize
              : ISLAND_SIZE[mode]);
        // Upright on a side: the bubble's length runs down the screen edge.
        const size =
          mode === "collapsed" && get().vertical ? { width: wanted.height, height: wanted.width } : wanted;
        const current = get();
        if (mode === current.mode && sameSize(current.size, size)) return;
        const requestId = ++latestRequest;
        // Never shrink here, only grow: the island may still be animating down from something
        // bigger (the bubble refits while it collapses). `settle` trims once it lands.
        const target = unionSize(
          windowFor(current.mode, current.size),
          windowSizeFor(mode, size, current.dockWidth, { vertical: current.vertical }),
        );
        await resizeTo(windowSize ? unionSize(windowSize, target) : target);
        // A newer request arrived while we were resizing; let it win.
        if (requestId !== latestRequest) return;
        // Anything that needs the island (a card, a click, a shortcut) brings it out of hiding.
        set(mode === "collapsed" ? { mode, size } : { mode, size, tucked: false });
      },

      toggleExpanded: () => get().request(get().mode === "expanded" ? "collapsed" : "expanded"),

      settle: () => resizeTo(windowFor(get().mode, get().size)),

      refit: () => {
        windowSize = null;
        return resizeTo(windowFor(get().mode, get().size));
      },
    };
  });
}
