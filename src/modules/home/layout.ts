// What Edit layout can rearrange on Home: its cards.
import type { LayoutGroup } from "../../shell/layout";

export const HOME_CARDS = [
  { id: "system", label: "System" },
  { id: "controls", label: "Volume & brightness" },
  { id: "screen", label: "Screen time" },
  { id: "weather", label: "Weather" },
] as const;

export const HOME_LAYOUT: readonly LayoutGroup[] = [{ id: "home", label: "Cards", items: HOME_CARDS }];
