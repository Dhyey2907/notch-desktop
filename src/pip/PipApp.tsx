// Picture-in-picture (#pip): another window drawn live by Windows into this small always-on-top
// window. Hover for the bar: drag it to move, Crop to keep just the video, play/pause, close.
import { invoke } from "@tauri-apps/api/core";
import { Crop, Pause, RotateCcw, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState, type PointerEvent } from "react";

import type { PipSource } from "../core/generated/PipSource";
import { applyCrop, cropKey, FULL, loadCrop, saveCrop, selectionCrop, type Frac } from "./crop";
import "./pip.css";

export function PipApp() {
  const [source, setSource] = useState<PipSource | null>(null);
  const [crop, setCrop] = useState<Frac>(FULL);
  const [cropping, setCropping] = useState(false);
  const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const view = useRef<HTMLDivElement>(null);

  const load = useCallback(() => {
    invoke<PipSource | null>("pip_source").then(
      (s) => {
        setSource(s);
        if (s) setCrop(loadCrop(cropKey(s.app)));
      },
      () => {},
    );
  }, []);

  useEffect(() => {
    load();
    window.addEventListener("overline-pip", load);
    return () => window.removeEventListener("overline-pip", load);
  }, [load]);

  // Ask Windows to draw the source over the view, cropped; again on every resize.
  useEffect(() => {
    const el = view.current;
    if (!el || !source) return;
    const place = () => {
      const r = el.getBoundingClientRect();
      const k = window.devicePixelRatio;
      void invoke("pip_place", {
        left: Math.round(r.left * k),
        top: Math.round(r.top * k),
        right: Math.round(r.right * k),
        bottom: Math.round(r.bottom * k),
        cropX: crop.x,
        cropY: crop.y,
        cropW: crop.w,
        cropH: crop.h,
        dim: cropping,
      }).catch(() => {});
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(el);
    return () => observer.disconnect();
  }, [source, crop, cropping]);

  const point = (e: PointerEvent<HTMLDivElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height };
  };

  const finishCrop = () => {
    if (drag && source) {
      const next = applyCrop(crop, selectionCrop(drag));
      setCrop(next);
      saveCrop(cropKey(source.app), next);
    }
    setDrag(null);
    setCropping(false);
  };

  return (
    <div className="pip">
      <div ref={view} className="pip__view" />
      {cropping && (
        <div
          className="pip__crop"
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            const p = point(e);
            setDrag({ x0: p.x, y0: p.y, x1: p.x, y1: p.y });
          }}
          onPointerMove={(e) => {
            if (!drag) return;
            const p = point(e);
            setDrag({ ...drag, x1: p.x, y1: p.y });
          }}
          onPointerUp={finishCrop}
        >
          {drag ? (
            <span
              className="pip__crop-box"
              style={{
                left: `${Math.min(drag.x0, drag.x1) * 100}%`,
                top: `${Math.min(drag.y0, drag.y1) * 100}%`,
                width: `${Math.abs(drag.x1 - drag.x0) * 100}%`,
                height: `${Math.abs(drag.y1 - drag.y0) * 100}%`,
              }}
            />
          ) : (
            <span className="pip__crop-hint">Drag over the part to keep</span>
          )}
        </div>
      )}
      <div className="pip__bar" data-tauri-drag-region>
        <span className="pip__title" data-tauri-drag-region>
          {source ? `${source.app} · ${source.title}` : "Nothing to show"}
        </span>
        <button
          type="button"
          aria-label="Play or pause"
          onClick={() => void invoke("media_command", { command: { kind: "playPause" } })}
        >
          <Pause size={13} aria-hidden />
        </button>
        <button
          type="button"
          aria-label="Crop"
          aria-pressed={cropping}
          onClick={() => setCropping((c) => !c)}
        >
          <Crop size={13} aria-hidden />
        </button>
        <button
          type="button"
          aria-label="Show the whole window"
          onClick={() => {
            setCrop(FULL);
            if (source) saveCrop(cropKey(source.app), FULL);
          }}
        >
          <RotateCcw size={13} aria-hidden />
        </button>
        <button type="button" aria-label="Close" onClick={() => void invoke("pip_close")}>
          <X size={14} aria-hidden />
        </button>
      </div>
    </div>
  );
}
