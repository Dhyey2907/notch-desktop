import { House } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";
import HomePeek from "./HomePeek";

export const homeModule: NotchModule = {
  id: "home",
  name: "Home",
  icon: House,
  order: 0,
  Peek: HomePeek,
  Expanded: lazy(() => import("./HomeExpanded")),
};
