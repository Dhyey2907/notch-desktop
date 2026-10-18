// Cropping the picture-in-picture view: fractions of the source window, nested crops, and the
// crop remembered per app. Pure apart from the storage helpers, so it's tested.

export interface Frac {
  x: number;
  y: number;
  w: number;
  h: number;
}

export const FULL: Frac = { x: 0, y: 0, w: 1, h: 1 };

const clamp = (v: number) => Math.max(0, Math.min(1, v));

/** A dragged box (fractions of the current view) as a crop, ignoring direction. */
export function selectionCrop(d: { x0: number; y0: number; x1: number; y1: number }): Frac {
  const x = clamp(Math.min(d.x0, d.x1));
  const y = clamp(Math.min(d.y0, d.y1));
  return { x, y, w: clamp(Math.max(d.x0, d.x1)) - x, h: clamp(Math.max(d.y0, d.y1)) - y };
}

/** Crop again inside the current crop. A box too small to mean anything keeps the current one. */
export function applyCrop(current: Frac, inner: Frac): Frac {
  if (inner.w < 0.03 || inner.h < 0.03) return current;
  return {
    x: current.x + inner.x * current.w,
    y: current.y + inner.y * current.h,
    w: inner.w * current.w,
    h: inner.h * current.h,
  };
}

export function cropKey(app: string): string {
  return `overline.pipCrop.${app.toLowerCase()}`;
}

export function loadCrop(key: string): Frac {
  try {
    const v = JSON.parse(localStorage.getItem(key) ?? "null") as Frac | null;
    return v && [v.x, v.y, v.w, v.h].every((n) => typeof n === "number" && n >= 0 && n <= 1) ? v : FULL;
  } catch {
    return FULL;
  }
}

export function saveCrop(key: string, crop: Frac) {
  try {
    localStorage.setItem(key, JSON.stringify(crop));
  } catch {
    // Storage off: the crop lasts until the window closes.
  }
}
