// Snap layouts for Desk → Layouts: each layout is a set of zones, as fractions of the screen's
// work area (the part not covered by the taskbar).
import type { Zone } from "../../core/generated/Zone";

export interface Layout {
  id: string;
  label: string;
  zones: Zone[];
}

const third = 1 / 3;

export const LAYOUTS: Layout[] = [
  {
    id: "halves",
    label: "Halves",
    zones: [
      { x: 0, y: 0, w: 0.5, h: 1 },
      { x: 0.5, y: 0, w: 0.5, h: 1 },
    ],
  },
  {
    id: "two-one",
    label: "Wide + side",
    zones: [
      { x: 0, y: 0, w: 2 * third, h: 1 },
      { x: 2 * third, y: 0, w: third, h: 1 },
    ],
  },
  {
    id: "thirds",
    label: "Thirds",
    zones: [
      { x: 0, y: 0, w: third, h: 1 },
      { x: third, y: 0, w: third, h: 1 },
      { x: 2 * third, y: 0, w: third, h: 1 },
    ],
  },
  {
    id: "side-stack",
    label: "1 + 2",
    zones: [
      { x: 0, y: 0, w: 0.5, h: 1 },
      { x: 0.5, y: 0, w: 0.5, h: 0.5 },
      { x: 0.5, y: 0.5, w: 0.5, h: 0.5 },
    ],
  },
  {
    id: "quarters",
    label: "Quarters",
    zones: [
      { x: 0, y: 0, w: 0.5, h: 0.5 },
      { x: 0.5, y: 0, w: 0.5, h: 0.5 },
      { x: 0, y: 0.5, w: 0.5, h: 0.5 },
      { x: 0.5, y: 0.5, w: 0.5, h: 0.5 },
    ],
  },
  { id: "center", label: "Centered", zones: [{ x: 0.15, y: 0.08, w: 0.7, h: 0.84 }] },
];

/** First guess at which window goes in each zone: the most recent windows, in order. */
export function defaultPicks(windowIds: number[], zones: number): (number | null)[] {
  return Array.from({ length: zones }, (_, i) => windowIds[i] ?? null);
}
