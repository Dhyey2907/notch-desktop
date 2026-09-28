// Shared motion presets (design.md §8). Components use these instead of ad-hoc timings.
import type { Transition } from "motion/react";

export type IslandMotionState = "collapsed" | "peek" | "expanded";

/** Island container springs: peek 180–250ms, expansion 350–500ms, collapse 250–400ms. */
export const ISLAND_SPRING: Record<IslandMotionState, Transition> = {
  collapsed: { type: "spring", visualDuration: 0.32, bounce: 0 },
  peek: { type: "spring", visualDuration: 0.22, bounce: 0.1 },
  expanded: { type: "spring", visualDuration: 0.42, bounce: 0.18 },
};

/** Serious notifications (approvals, errors) never bounce. */
export const SERIOUS_SPRING: Transition = { type: "spring", visualDuration: 0.3, bounce: 0 };

/** Tiny interactions: icon presses, hover fills (120–180ms). */
export const MICRO: Transition = { duration: 0.15, ease: [0.2, 0, 0, 1] };

/** Larger content swaps inside a surface, e.g. switching modules (300–500ms). */
export const CONTENT_SWAP: Transition = { type: "spring", visualDuration: 0.32, bounce: 0 };

export const INSTANT: Transition = { duration: 0 };

/**
 * Content enters after its container has started to unfold, so the surface leads and the
 * content follows.
 */
export function contentEnter(state: IslandMotionState, reduceMotion: boolean | null): Transition {
  if (reduceMotion) return INSTANT;
  return { delay: state === "expanded" ? 0.1 : 0.05, duration: 0.2 };
}
