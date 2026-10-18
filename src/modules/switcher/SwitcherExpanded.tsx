// The window switcher: a grid of every open window, each drawn live by Windows (DWM) over its
// tile. Arrows move, Enter or a click brings that window forward.
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import type { DeskWindow } from "../../core/generated/DeskWindow";
import { ipc } from "../../core/ipc";
import { EmptyState, ModuleContainer } from "../../design/components";
import { uiScale } from "../../shell/appearance";
import { moveIndex } from "./grid";
import "./switcher.css";

const MAX_TILES = 12;
export default function SwitcherExpanded() {
  const [windows, setWindows] = useState<DeskWindow[] | null>(null);
  const [index, setIndex] = useState(0);
  const views = useRef<(HTMLSpanElement | null)[]>([]);
  const grid = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let live = true;
    ipc.deskWindows().then(
      (list) => live && setWindows(list.slice(0, MAX_TILES)),
      () => live && setWindows([]),
    );
    return () => {
      live = false;
    };
  }, []);

  const ids = useMemo(() => (windows ?? []).map((w) => w.id).join(","), [windows]);

  // Ask Windows to draw each window over its tile; again once the island settles or resizes.
  useEffect(() => {
    if (!windows || windows.length === 0) return;
    const place = () => {
      const k = window.devicePixelRatio * uiScale();
      const tiles = windows.flatMap((w, i) => {
        const el = views.current[i];
        if (!el) return [];
        const r = el.getBoundingClientRect();
        if (r.width < 4 || r.height < 4) return [];
        return [
          {
            id: w.id,
            left: Math.round(r.left * k),
            top: Math.round(r.top * k),
            right: Math.round(r.right * k),
            bottom: Math.round(r.bottom * k),
          },
        ];
      });
      void ipc.deskGridShow(tiles).catch(() => {});
    };
    place();
    const settle = window.setTimeout(place, 450);
    const observer = new ResizeObserver(place);
    if (grid.current) observer.observe(grid.current);
    return () => {
      window.clearTimeout(settle);
      observer.disconnect();
      void ipc.deskGridHide().catch(() => {});
    };
    // `ids` stands for `windows`: re-place only when the set of windows changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ids]);

  useEffect(() => {
    grid.current?.focus();
  }, [ids]);

  const go = (w: DeskWindow | undefined) => {
    if (w) void ipc.deskFocus(w.id).catch(() => {});
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const count = windows?.length ?? 0;
    if (e.key.startsWith("Arrow")) {
      e.preventDefault();
      e.stopPropagation();
      setIndex((i) => moveIndex(i, e.key, count));
    } else if (e.key === "Enter") {
      e.preventDefault();
      go(windows?.[index]);
    }
  };

  return (
    <ModuleContainer title="Windows">
      {windows && windows.length === 0 ? (
        <EmptyState title="No windows open" body="Open apps show up here as live views." />
      ) : (
        <div
          ref={grid}
          className="switcher"
          role="listbox"
          aria-label="Open windows"
          tabIndex={0}
          onKeyDown={onKeyDown}
        >
          {(windows ?? []).map((w, i) => (
            <button
              key={w.id}
              type="button"
              role="option"
              aria-selected={i === index}
              className={`switcher__tile${i === index ? " switcher__tile--on" : ""}`}
              title={`${w.app} · ${w.title}`}
              onMouseEnter={() => setIndex(i)}
              onClick={() => go(w)}
            >
              <span
                className="switcher__view"
                ref={(el) => {
                  views.current[i] = el;
                }}
              />
              <span className="switcher__name">{w.app}</span>
            </button>
          ))}
        </div>
      )}
    </ModuleContainer>
  );
}
