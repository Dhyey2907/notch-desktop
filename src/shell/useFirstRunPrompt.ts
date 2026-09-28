import { useEffect } from "react";

import { useActivities } from "../activity";
import { ipc } from "../core/ipc";
import { useSettings } from "../settings";

/**
 * First run: ask before registering with Windows startup (architecture.md §2: startup
 * behaviour is explicit user control). Shown once as an activity; either answer is final
 * until changed in Settings → General.
 */
export function useFirstRunPrompt() {
  const pending = useSettings((s) => s.settings !== null && !s.settings.firstRunCompleted);

  useEffect(() => {
    if (!pending) return;
    const complete = () => useSettings.getState().update((s) => ({ ...s, firstRunCompleted: true }));
    const id = useActivities.getState().push({
      moduleId: "home",
      priority: "normal",
      persistent: true,
      dedupKey: "first-run",
      title: "Start Notch Desktop with Windows?",
      body: "You can change this anytime in Settings.",
      actions: [
        { id: "later", label: "Not now", run: complete },
        {
          id: "enable",
          label: "Start with Windows",
          variant: "primary",
          run: async () => {
            await ipc.autostartSet(true);
            await complete();
          },
        },
      ],
    });
    return () => useActivities.getState().dismiss(id);
  }, [pending]);
}
