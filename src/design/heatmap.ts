import type { HeatmapCell } from "./components";

/** Bucket raw values into heatmap levels relative to the busiest day. */
export function heatmapLevel(value: number, max: number): HeatmapCell["level"] {
  if (value <= 0 || max <= 0) return 0;
  const ratio = value / max;
  if (ratio > 0.75) return 4;
  if (ratio > 0.5) return 3;
  if (ratio > 0.25) return 2;
  return 1;
}
