import { useState } from "react";

import { useActivities } from "../../activity";
import { ipc } from "../../core/ipc";
import type { DevTestActivity } from "../../core/generated/DevTestActivity";
import { Button, MockBadge, ModuleContainer, Surface } from "../../design/components";
import { useIsland } from "../../shell/useIsland";

/** Long enough to collapse the island first, so the activity has somewhere to appear. */
const DELAY_MS = 1200;

const PRESETS: { label: string; activities: DevTestActivity[] }[] = [
  {
    label: "Low",
    activities: [
      {
        priority: "low",
        title: "Low priority",
        body: "Fades after 3s",
        persistent: false,
        actionable: false,
      },
    ],
  },
  {
    label: "Normal",
    activities: [
      {
        priority: "normal",
        title: "Sample message",
        body: "Fades after 5s",
        persistent: false,
        actionable: false,
      },
    ],
  },
  {
    label: "High",
    activities: [
      {
        priority: "high",
        title: "Meeting in 5 minutes",
        body: "Fades after 8s",
        persistent: false,
        actionable: false,
      },
    ],
  },
  {
    label: "Critical + actions",
    activities: [
      {
        priority: "critical",
        title: "Wants to run: pnpm test",
        body: "Persists until you act",
        persistent: true,
        actionable: true,
      },
    ],
  },
  {
    label: "Burst ×3",
    activities: ["One", "Two", "Three"].map((n) => ({
      priority: "normal" as const,
      title: `Burst ${n}`,
      body: "Stacks as a count",
      persistent: false,
      actionable: false,
    })),
  },
  {
    label: "Replace (same key)",
    activities: [
      {
        priority: "normal",
        title: "Now playing: Track A",
        body: "Replaced in place",
        dedupKey: "dev-track",
        persistent: false,
        actionable: false,
      },
      {
        priority: "normal",
        title: "Now playing: Track B",
        body: "Replaced in place",
        dedupKey: "dev-track",
        persistent: false,
        actionable: false,
      },
    ],
  },
];

export default function DevEventsExpanded() {
  const [sent, setSent] = useState<string | null>(null);

  const fire = (label: string, activities: DevTestActivity[]) => {
    activities.forEach((activity, i) => void ipc.devEmitTestActivity(activity, DELAY_MS + i * 150));
    setSent(label);
    void useIsland.getState().request("collapsed");
  };

  return (
    <ModuleContainer title="Dev events" trailing={<MockBadge />}>
      <Surface>
        <p className="dev-events__hint">
          Fires fake events through the Rust event bus after {DELAY_MS / 1000}s, then collapses the island.
          {sent && ` Last: ${sent}.`}
        </p>
        <div className="gallery__row">
          {PRESETS.map((preset) => (
            <Button key={preset.label} onClick={() => fire(preset.label, preset.activities)}>
              {preset.label}
            </Button>
          ))}
          <Button
            onClick={() => {
              void ipc.devEmitTestApproval("Bash", { command: "pnpm test" }, DELAY_MS);
              setSent("Approval");
              void useIsland.getState().request("collapsed");
            }}
          >
            Approval
          </Button>
          <Button
            onClick={() => {
              void ipc.devEmitTestApproval("Bash", { command: "git push --force origin main" }, DELAY_MS);
              setSent("Destructive approval");
              void useIsland.getState().request("collapsed");
            }}
          >
            Destructive approval
          </Button>
          <Button variant="danger" onClick={() => useActivities.getState().clear()}>
            Clear all
          </Button>
        </div>
      </Surface>
    </ModuleContainer>
  );
}
