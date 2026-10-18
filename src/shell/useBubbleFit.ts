import { useLayoutEffect, type RefObject } from "react";

import { ISLAND_SIZE } from "./geometry";
import { useIsland } from "./useIsland";

/** Room at each end of the bubble (its rounded ends). */
const END_PADDING = 9;
const GAP = 6;
const MAX_WIDTH = 160;
/** With a widget pinned beside the bubble it may grow this wide. */
export const MAX_WIDTH_WITH_WIDGET = 320;

/** The resting bubble's width for marks of these widths (logical px). */
export function bubbleWidthFor(widths: number[], max = MAX_WIDTH): number {
  const shown = widths.filter((w) => Number.isFinite(w) && w > 0);
  if (shown.length === 0) return ISLAND_SIZE.collapsed.width;
  const content = shown.reduce((a, b) => a + b, 0) + GAP * (shown.length - 1);
  return Math.min(max, Math.max(ISLAND_SIZE.collapsed.width, Math.ceil(content + END_PADDING * 2)));
}

/**
 * Grow the resting bubble so its marks (music, downloads, Claude, the pet…) never crowd or
 * cover each other; shrink it back when they go.
 */
export function useBubbleFit(island: RefObject<HTMLElement | null>, active: boolean, dot: boolean) {
  // The dot style is always the same small size.
  useLayoutEffect(() => {
    if (dot) void useIsland.getState().setBubbleWidth(ISLAND_SIZE.collapsed.width);
  }, [dot]);

  useLayoutEffect(() => {
    const root = island.current;
    if (dot) return;
    if (!root || !active) return;

    // Marks can be SVGs (no offsetWidth): measure on screen and undo the notch-size zoom.
    const size = (el: Element) => {
      const zoom = root.offsetWidth > 0 ? root.getBoundingClientRect().width / root.offsetWidth : 1;
      return el.getBoundingClientRect().width / (zoom || 1);
    };
    const widthOf = (content: HTMLElement) => {
      const marks: number[] = [];
      for (const child of Array.from(content.children)) {
        // Overlays (the workday line) don't take room in the row.
        if (getComputedStyle(child).position === "absolute") continue;
        if (child.classList.contains("compact-host")) {
          for (const mark of Array.from(child.children)) marks.push(size(mark));
        } else {
          marks.push(size(child));
        }
      }
      return bubbleWidthFor(marks, content.querySelector(".mini-widget") ? MAX_WIDTH_WITH_WIDGET : undefined);
    };
    const contents = () => Array.from(root.querySelectorAll<HTMLElement>(".island-content--collapsed"));
    // While the old content fades out the new one is already there: fit the larger.
    const fit = () => {
      const width = Math.max(ISLAND_SIZE.collapsed.width, ...contents().map(widthOf));
      void useIsland.getState().setBubbleWidth(width);
    };
    const resize = new ResizeObserver(fit);
    const watch = () => {
      resize.disconnect();
      for (const c of contents())
        for (const el of Array.from(c.querySelectorAll<HTMLElement>(":scope > *, .compact-host > *")))
          resize.observe(el);
      fit();
    };

    watch();
    const mutations = new MutationObserver(watch);
    mutations.observe(root, { childList: true, subtree: true });
    return () => {
      resize.disconnect();
      mutations.disconnect();
    };
  }, [island, active, dot]);
}
