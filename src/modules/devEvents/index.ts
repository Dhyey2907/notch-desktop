// Development-only module that turns backend `DevTestActivity` events into live activities,
// proving the full adapter → event → activity engine → island path without real
// integrations (plan.md Phase 4 acceptance). Excluded from release builds.
import { FlaskConical } from "lucide-react";
import { lazy } from "react";

import type { ModuleContext, NotchModule } from "../types";

function init({ events, activities }: ModuleContext) {
  return events.on("DevTestActivity", ({ event: { payload } }) => {
    activities.push({
      moduleId: "devEvents",
      priority: payload.priority,
      title: `${payload.title} · MOCK`,
      body: payload.body,
      dedupKey: payload.dedupKey,
      persistent: payload.persistent,
      actions: payload.actionable
        ? [
            { id: "dismiss", label: "Dismiss", run: () => {} },
            {
              id: "approve",
              label: "Approve",
              confirmLabel: "Confirm?",
              variant: "warning",
              run: () => console.info("dev activity approved"),
            },
          ]
        : undefined,
    });
  });
}

export const devEventsModule: NotchModule = {
  id: "devEvents",
  name: "Dev events",
  icon: FlaskConical,
  order: 910,
  devOnly: true,
  init,
  // Marked pure so the bundler can drop this module from release builds.
  Expanded: /* @__PURE__ */ lazy(() => import("./DevEventsExpanded")),
};
