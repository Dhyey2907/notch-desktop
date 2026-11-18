// Drag to rearrange, right in the panel: hold a card for a moment, it lifts, and moving the
// pointer moves it among the others. Letting go saves the order (the same order Edit layout
// shows); Esc puts it back. A press that moves before the hold ends is a normal press (a
// slider, a scroll), so cards keep working as before.
import { motion } from "motion/react";
import { useEffect, useRef, useState, type PointerEvent, type ReactNode } from "react";

import { useReduceMotion } from "../design/useReduceMotion";
import { LONG_PRESS_MS, moveId, PRESS_SLOP_PX } from "./dockEdit";
import { ordered, savePanel, usePanelLayout } from "./layout";
import { nearestUnit, savedOrder } from "./arrange";
import "./arrange.css";

/** Something you can drag: one card, or a row of cards that move together. */
export interface ArrangeUnit {
  key: string;
  /** The layout ids it stands for, in order. */
  ids: readonly string[];
  node: ReactNode;
}

export function Arrange({
  group,
  items,
  units,
  className,
  label,
}: {
  /** The layout group (`savePanel` id). */
  group: string;
  /** Everything the group can hold, shown or hidden. */
  items: readonly { id: string }[];
  units: readonly ArrangeUnit[];
  className?: string;
  label?: string;
}) {
  const layout = usePanelLayout(group);
  const reduceMotion = useReduceMotion();
  const [draft, setDraft] = useState<string[] | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const press = useRef<{ key: string; id: number; x: number; y: number; timer: number } | null>(null);
  const swallowClick = useRef(false);
  const refs = useRef(new Map<string, HTMLElement>());

  const byKey = new Map(units.map((u) => [u.key, u]));
  const shown = draft ? draft.flatMap((k) => byKey.get(k) ?? []) : units;

  const cancelPress = () => {
    if (press.current) window.clearTimeout(press.current.timer);
    press.current = null;
  };

  const finish = (save: boolean) => {
    cancelPress();
    const order = draft;
    setDragging(null);
    setDraft(null);
    if (!save || !order) return;
    const next = order.flatMap((k) => byKey.get(k) ?? []);
    const all = ordered(items, layout).map((i) => i.id);
    void savePanel(group, { order: savedOrder(next, all) });
  };

  // Esc while dragging puts everything back.
  useEffect(() => {
    if (!dragging) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      finish(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });

  useEffect(() => () => cancelPress(), []);

  const onPointerDown = (key: string) => (e: PointerEvent<HTMLElement>) => {
    if (e.button !== 0 || units.length < 2) return;
    const el = e.currentTarget;
    const pointerId = e.pointerId;
    cancelPress();
    press.current = {
      key,
      id: pointerId,
      x: e.clientX,
      y: e.clientY,
      timer: window.setTimeout(() => {
        press.current = null;
        swallowClick.current = true;
        try {
          el.setPointerCapture(pointerId);
        } catch {
          // The pointer is already gone; the drag simply ends on the next event.
        }
        setDraft(units.map((u) => u.key));
        setDragging(key);
      }, LONG_PRESS_MS),
    };
  };

  const onPointerMove = (e: PointerEvent<HTMLElement>) => {
    const p = press.current;
    if (p && e.pointerId === p.id) {
      if (Math.hypot(e.clientX - p.x, e.clientY - p.y) > PRESS_SLOP_PX) cancelPress();
      return;
    }
    if (!dragging || !draft) return;
    const centers = draft.map((k) => {
      const r = refs.current.get(k)?.getBoundingClientRect();
      return r ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : { x: Infinity, y: Infinity };
    });
    const at = nearestUnit({ x: e.clientX, y: e.clientY }, centers);
    if (at === -1 || draft[at] === dragging) return;
    setDraft(moveId(draft, dragging, at));
  };

  const onPointerUp = () => {
    if (dragging) finish(true);
    else cancelPress();
  };

  return (
    <div
      className={`arrange${dragging ? " arrange--dragging" : ""}${className ? ` ${className}` : ""}`}
      aria-label={label}
    >
      {shown.map((unit) => (
        <motion.div
          key={unit.key}
          ref={(el: HTMLDivElement | null) => {
            if (el) refs.current.set(unit.key, el);
            else refs.current.delete(unit.key);
          }}
          layout={reduceMotion ? false : "position"}
          transition={{ type: "spring", visualDuration: 0.22, bounce: 0.1 }}
          className={`arrange__item${dragging === unit.key ? " arrange__item--lifted" : ""}`}
          title={units.length > 1 ? "Hold to drag" : undefined}
          onPointerDown={onPointerDown(unit.key)}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={() => finish(false)}
          onClickCapture={(e) => {
            if (!swallowClick.current) return;
            swallowClick.current = false;
            e.stopPropagation();
            e.preventDefault();
          }}
        >
          {unit.node}
        </motion.div>
      ))}
    </div>
  );
}
