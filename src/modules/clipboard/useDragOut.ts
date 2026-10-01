import { useRef, type PointerEvent } from "react";

import { useFileDrag } from "../../shell/fileDrag";

/** Pointer travel before a press becomes a drag. */
const DRAG_THRESHOLD = 6;

/** Press and move: `start` drags files out of the notch (Windows takes over the mouse). */
export function useDragOut(start: () => Promise<unknown>, onError: (error: unknown) => void) {
  const from = useRef<{ x: number; y: number } | null>(null);
  return {
    onPointerDown: (e: PointerEvent) => {
      if (e.button !== 0) return;
      from.current = { x: e.clientX, y: e.clientY };
      // Keep getting moves after the pointer leaves a small target (the "Drag all" grip).
      e.currentTarget.setPointerCapture(e.pointerId);
    },
    onPointerMove: (e: PointerEvent) => {
      const origin = from.current;
      if (!origin) return;
      if ((e.buttons & 1) === 0) {
        from.current = null;
        return;
      }
      if (Math.hypot(e.clientX - origin.x, e.clientY - origin.y) < DRAG_THRESHOLD) return;
      from.current = null;
      e.currentTarget.releasePointerCapture(e.pointerId);
      useFileDrag.setState({ draggingOut: true });
      start()
        .catch(onError)
        .finally(() => useFileDrag.setState({ draggingOut: false }));
    },
    onPointerUp: () => {
      from.current = null;
    },
  };
}
