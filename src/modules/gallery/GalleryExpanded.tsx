import { useState } from "react";

import {
  Button,
  ConfirmButton,
  EmptyState,
  ErrorState,
  Heatmap,
  MediaControl,
  MockBadge,
  ModuleContainer,
  NotificationCard,
  Pill,
  Progress,
  Surface,
  type HeatmapCell,
} from "../../design/components";
import { heatmapLevel } from "../../design/heatmap";

// Deterministic sample data so screenshots are stable. Clearly marked MOCK in the UI.
function sampleCells(): HeatmapCell[] {
  const start = Date.UTC(2026, 0, 5);
  const values = Array.from({ length: 26 * 7 }, (_, i) =>
    Math.max(0, Math.round(Math.sin(i / 3) * 6 + (i % 5))),
  );
  const max = Math.max(...values);
  return values.map((value, i) => ({
    date: new Date(start + i * 86_400_000).toISOString().slice(0, 10),
    value,
    level: heatmapLevel(value, max),
  }));
}

export default function GalleryExpanded() {
  const [playing, setPlaying] = useState(false);
  const [confirmed, setConfirmed] = useState(0);

  return (
    <ModuleContainer title="Design gallery" trailing={<MockBadge />}>
      <div className="gallery">
        <Surface>
          <div className="gallery__row">
            <Pill>Neutral</Pill>
            <Pill tone="success">Success</Pill>
            <Pill tone="warning">Warning</Pill>
            <Pill tone="danger">Danger</Pill>
            <Pill tone="info">Info</Pill>
          </div>
          <div className="gallery__row">
            <Button variant="primary">Primary</Button>
            <Button>Secondary</Button>
            <ConfirmButton confirmLabel="Confirm push?" onConfirm={() => setConfirmed((n) => n + 1)}>
              Push {confirmed > 0 && `(${confirmed})`}
            </ConfirmButton>
            <ConfirmButton variant="danger" confirmLabel="Really delete?" onConfirm={() => {}}>
              Delete
            </ConfirmButton>
          </div>
        </Surface>

        <Surface>
          <NotificationCard avatar="S" title="Sample sender" body="Sample message preview text" time="now" />
          <Progress label="Sample progress" value={0.63} tone="info" />
          <MediaControl playing={playing} onPlayPause={() => setPlaying((p) => !p)} canPrevious={false} />
        </Surface>

        <Surface>
          <Heatmap cells={sampleCells()} label="Sample activity heatmap" unit="events" />
        </Surface>

        <Surface>
          <EmptyState
            title="Your day lives here."
            body="Connect a calendar to see meetings and reminders."
            action={<Button>Connect Calendar</Button>}
          />
        </Surface>

        <Surface>
          <ErrorState
            title="Calendar disconnected"
            body="Reconnect your account to restore meeting and reminder updates."
            action={<Button>Reconnect</Button>}
          />
        </Surface>
      </div>
    </ModuleContainer>
  );
}
