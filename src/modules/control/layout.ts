// What Edit layout can rearrange in Control: its sections, the round switches, the display chips.
import type { LayoutGroup } from "../../shell/layout";

export const SECTIONS = [
  { id: "toggles", label: "Quick switches" },
  { id: "display", label: "Display chips" },
  { id: "network", label: "Network card" },
  { id: "battery", label: "Battery card" },
  { id: "devices", label: "Accessory batteries" },
  { id: "more", label: "More buttons" },
] as const;

export const TOGGLES = [
  { id: "wifi", label: "Wi-Fi" },
  { id: "bluetooth", label: "Bluetooth" },
  { id: "hotspot", label: "Hotspot" },
  { id: "airplane", label: "Airplane" },
] as const;

export const DISPLAY = [
  { id: "night-light", label: "Night light" },
  { id: "hdr", label: "HDR" },
  { id: "refresh", label: "Refresh rate" },
  { id: "touchpad", label: "Touchpad" },
] as const;

export const CONTROL_LAYOUT: readonly LayoutGroup[] = [
  { id: "control", label: "Sections", items: SECTIONS, keepOne: true },
  { id: "control.toggles", label: "Quick switches", items: TOGGLES },
  { id: "control.display", label: "Display chips", items: DISPLAY },
];
