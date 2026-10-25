// Rewrite selection (Ctrl+Alt+Shift+R): select text in any app, press the shortcut, pick a style. It
// isn't in the dock; the shortcut opens it with the selection.
import { WandSparkles } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";

export const rewriteModule: NotchModule = {
  id: "rewrite",
  name: "Rewrite",
  icon: WandSparkles,
  order: 997,
  alwaysOn: true,
  dockless: true,
  Expanded: lazy(() => import("./RewriteExpanded")),
};
