// Moving the notch by hand, and getting it out of the way:
// - Alt+drag the resting bubble anywhere: it shows on the screen edge nearest the cursor (top,
//   bottom, left or right, on any screen) and stays where you let go. A plain press never
//   moves it.
// - "Fade while typing": Overline says when you're typing just under it, and the bubble fades.
import { useEffect, useRef, useState, type PointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

import { eventBus } from "../core/eventBus";
import type { NotchSpot } from "../core/generated/NotchSpot";
import { windowLabel } from "../core/window";
import { useSettings } from "../settings";

/** Moving less than this is still a click. */
const DRAG_PX = 6;
/** Previews are sent at most this often while dragging. */
const PREVIEW_MS = 30;

let dragging = false;
/** True while the notch is being dragged (hovering mustn't open it meanwhile). */
export const isNotchDragging = () => dragging;

/** The settings for where a drag landed: the edge, and the place along it. */
export function spotPatch(spot: NotchSpot): {
  edge: NotchSpot["edge"];
  offsetX?: number;
  offsetSide?: number;
} {
  return spot.edge === "left" || spot.edge === "right"
    ? { edge: spot.edge, offsetSide: spot.along }
    : { edge: spot.edge, offsetX: spot.along };
}

const drag = (phase: "move" | "end" | "clear") =>
  invoke<NotchSpot | null>("shell_drag_notch", { phase, keepEdge: false }).catch(() => null);

/** `onStart`: called once a press turns into a drag (e.g. to cancel a pending hover-open). */
export function useNotchDrag(onStart?: () => void) {
  const press = useRef<{
    id: number;
    x: number;
    y: number;
    moved: boolean;
    sent: number;
  } | null>(null);
  const swallowClick = useRef(false);

  const onPointerDown = (e: PointerEvent<HTMLElement>) => {
    if (e.button !== 0 || !e.altKey) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    press.current = { id: e.pointerId, x: e.screenX, y: e.screenY, moved: false, sent: 0 };
  };

  const onPointerMove = (e: PointerEvent<HTMLElement>) => {
    const p = press.current;
    if (!p || e.pointerId !== p.id) return;
    if (!p.moved) {
      if (Math.hypot(e.screenX - p.x, e.screenY - p.y) < DRAG_PX) return;
      p.moved = true;
      dragging = true;
      onStart?.();
    }
    const now = performance.now();
    if (now - p.sent < PREVIEW_MS) return;
    p.sent = now;
    void drag("move");
  };

  const onPointerUp = (e: PointerEvent<HTMLElement>) => {
    const p = press.current;
    press.current = null;
    if (!p || e.pointerId !== p.id || !p.moved) return;
    swallowClick.current = true;
    void (async () => {
      const spot = e.type === "pointercancel" ? null : await drag("end");
      if (spot) {
        const patch = spotPatch(spot);
        // Dropped on another screen: the notch lives there from now on.
        await useSettings.getState().update((s) => ({
          ...s,
          appearance: { ...s.appearance, ...patch },
          display: spot.screen ? { ...s.display, mode: "one", monitor: spot.screen } : s.display,
        }));
      }
      await drag("clear");
      dragging = false;
    })();
  };

  /** True once after a drag: the click that ends it shouldn't open the notch. */
  const consumeClick = () => {
    const was = swallowClick.current;
    swallowClick.current = false;
    return was;
  };

  return { onPointerDown, onPointerMove, onPointerUp, onPointerCancel: onPointerUp, consumeClick };
}

/** Whether this island's bubble should fade (you're typing just under it). */
export function useNotchFade(): boolean {
  const [faded, setFaded] = useState(false);
  useEffect(
    () =>
      eventBus.on("NotchFade", ({ event }) => {
        if (event.payload.label === windowLabel()) setFaded(event.payload.on);
      }),
    [],
  );
  return faded;
}
