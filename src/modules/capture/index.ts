// Quick capture: one line from anywhere (Ctrl+Alt+Space) filed where it belongs — a task, a
// reminder, a note, a calendar event or a link to read later.
import { Zap } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";

export const captureModule: NotchModule = {
  id: "capture",
  name: "Quick capture",
  icon: Zap,
  order: 48,
  Expanded: lazy(() => import("./CaptureExpanded")),
};
