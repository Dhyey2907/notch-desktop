// The command palette (Ctrl+Alt+K): one place to run anything. It isn't in the dock; the
// shortcut opens it.
import { Command } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";

export const paletteModule: NotchModule = {
  id: "palette",
  name: "Commands",
  icon: Command,
  order: 999,
  alwaysOn: true,
  dockless: true,
  Expanded: lazy(() => import("./PaletteExpanded")),
};
