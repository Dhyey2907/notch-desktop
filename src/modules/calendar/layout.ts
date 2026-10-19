// What Edit layout can rearrange in Calendar: the month card and the agenda.
import type { LayoutGroup } from "../../shell/layout";

export const CALENDAR_PARTS = [
  { id: "month", label: "Month" },
  { id: "agenda", label: "Today & upcoming" },
] as const;

export const CALENDAR_LAYOUT: readonly LayoutGroup[] = [
  { id: "calendar", label: "Sections", items: CALENDAR_PARTS, keepOne: true },
];
