// Moving the notch by hand, and getting it out of the way:
// - Alt+drag the resting bubble sideways to slide it along the edge (saved when you let go).
// - "Fade while typing": Overline says when you're typing just under it, and the bubble fades.
import { useEffect, useRef, useState, type PointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

import { eventBus } from "../core/eventBus";
import { windowLabel } from "../core/window";
import { useSettings } from "../settings";

/** Moving less than this is still a click. */
const DRAG_PX = 4;
/** Previews are sent at most this often while dragging. */
const PREVIEW_MS = 30;

/** The offset a drag lands on: where it started plus how far the pointer went, kept in range. */
export function draggedOffset(start: number, dx: number): number {
  return Math.max(-4000, Math.min(4000, Math.round(start + dx)));
}

export function useNotchDrag() {
  const drag = useRef<{ id: number; x: number; start: number; moved: boolean; sent: number } | null>(null);
  const swallowClick = useRef(false);

  const onPointerDown = (e: PointerEvent<HTMLElement>) => {
    if (!e.altKey || e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture(e.pointerId);
    const start = useSettings.getState().settings?.appearance.offsetX ?? 0;
    drag.current = { id: e.pointerId, x: e.screenX, start, moved: false, sent: 0 };
  };

  const onPointerMove = (e: PointerEvent<HTMLElement>) => {
    const d = drag.current;
    if (!d || e.pointerId !== d.id) return;
    const dx = e.screenX - d.x;
    if (!d.moved && Math.abs(dx) < DRAG_PX) return;
    d.moved = true;
    const now = performance.now();
    if (now - d.sent < PREVIEW_MS) return;
    d.sent = now;
    void invoke("shell_preview_offset", { offsetX: draggedOffset(d.start, dx) }).catch(() => {});
  };

  const onPointerUp = (e: PointerEvent<HTMLElement>) => {
    const d = drag.current;
    drag.current = null;
    if (!d || e.pointerId !== d.id) return;
    if (!d.moved) return;
    swallowClick.current = true;
    const offsetX = draggedOffset(d.start, e.screenX - d.x);
    void useSettings
      .getState()
      .update((s) => ({ ...s, appearance: { ...s.appearance, offsetX } }))
      .finally(() => void invoke("shell_preview_offset", { offsetX: null }).catch(() => {}));
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
