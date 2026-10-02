// Decision maker: coin, dice and a spin-the-wheel of your own options.
import { Dices } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";

export const decideModule: NotchModule = {
  id: "decide",
  name: "Decide",
  icon: Dices,
  order: 95,
  Expanded: lazy(() => import("./DecideExpanded")),
};
