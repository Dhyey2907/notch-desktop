// The window switcher (Ctrl+Alt+W): every open window as a live view in the notch. It isn't in
// the dock; the shortcut (or the palette) opens it.
import { AppWindow } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";

export const switcherModule: NotchModule = {
  id: "switcher",
  name: "Windows",
  icon: AppWindow,
  order: 998,
  alwaysOn: true,
  dockless: true,
  Expanded: lazy(() => import("./SwitcherExpanded")),
};
