// Desk: Windows | Routines.
import { MonitorPlay, Pin, PinOff, Play, Settings2 } from "lucide-react";
import { useState } from "react";

import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer, SegmentedControl } from "../../design/components";
import { useSettings } from "../../settings";
import { usePolled } from "../devtools/usePolled";
import { showMini } from "./mini";
// The list rows share the Clipboard module's look (styles only).
import "../clipboard/clipboard.css";
import "./desk.css";

type Tab = "windows" | "routines";

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "That didn't work.";
}

export default function DeskExpanded() {
  const [tab, setTab] = useState<Tab>("windows");
  return (
    <ModuleContainer
      title="Desk"
      trailing={
        <SegmentedControl<Tab>
          label="Desk view"
          value={tab}
          options={[
            { value: "windows", label: "Windows" },
            { value: "routines", label: "Routines" },
          ]}
          onChange={setTab}
        />
      }
    >
      {tab === "windows" ? <Windows /> : <Routines />}
    </ModuleContainer>
  );
}

function Windows() {
  const windows = usePolled(() => ipc.deskWindows(), 3000);
  const [status, setStatus] = useState<string | null>(null);
  // Opacity follows the slider right away; the list catches up on the next poll.
  const [opacity, setOpacity] = useState<Record<number, number>>({});
  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  if (!windows) return <div className="desk" />;
  return (
    <div className="desk">
      {status && <p className="clips-status">{status}</p>}
      <ul className="clips-list" aria-label="Open windows">
        {windows.map((w) => {
          const shown = opacity[w.id] ?? w.opacity;
          return (
            <li key={w.id} className="clip">
              <button
                type="button"
                className="clip__main"
                title="Bring to front"
                onClick={() => run(ipc.deskFocus(w.id))}
              >
                <span className="clip__icon" aria-hidden>
                  {w.app.charAt(0).toUpperCase()}
                </span>
                <span className="clip__text">
                  <span className="clip__title">{w.title}</span>
                  <span className="clip__meta">
                    {w.app}
                    {w.onTop ? " · on top" : ""}
                    {shown < 100 ? ` · ${shown}% visible` : ""}
                  </span>
                </span>
              </button>
              <div className="clip__actions">
                <input
                  type="range"
                  className="desk-opacity"
                  min={20}
                  max={100}
                  step={5}
                  value={shown}
                  aria-label={`See-through: ${w.title}`}
                  title="See-through"
                  onChange={(e) => {
                    const v = Number(e.currentTarget.value);
                    setOpacity({ ...opacity, [w.id]: v });
                    void ipc.deskSetOpacity(w.id, v).catch((err) => setStatus(message(err)));
                  }}
                />
                <IconButton
                  icon={w.onTop ? PinOff : Pin}
                  label={w.onTop ? `Unpin ${w.title}` : `Keep ${w.title} on top`}
                  size="sm"
                  active={w.onTop}
                  onClick={() => run(ipc.deskSetOnTop(w.id, !w.onTop))}
                />
                <IconButton
                  icon={MonitorPlay}
                  label={`Watch ${w.title} live in the notch`}
                  size="sm"
                  onClick={() => showMini(w)}
                />
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function Routines() {
  const routines = useSettings((s) => s.settings?.desk.routines ?? []);
  const [status, setStatus] = useState<string | null>(null);
  const [running, setRunning] = useState<string | null>(null);
  if (routines.length === 0) {
    return (
      <EmptyState
        title="No routines yet"
        body="A routine opens your apps, sites and folders, closes clutter and can start focus — in one click or on a schedule. Make one in Settings → Desk."
        action={<Button onClick={() => void ipc.openSettingsWindow("desk")}>Make a routine</Button>}
      />
    );
  }
  return (
    <div className="desk">
      {status && <p className="clips-status">{status}</p>}
      <div className="desk-routines">
        {routines.map((r) => (
          <button
            key={r.id}
            type="button"
            className="desk-routine"
            disabled={running !== null}
            onClick={() => {
              setRunning(r.id);
              void ipc
                .routineRun(r.id)
                .then(
                  () => setStatus(null),
                  (e) => setStatus(message(e)),
                )
                .finally(() => setRunning(null));
            }}
          >
            <Play size={14} aria-hidden />
            <span className="desk-routine__name">{running === r.id ? "Running…" : r.name}</span>
            <span className="desk-routine__meta">
              {r.steps.length} {r.steps.length === 1 ? "step" : "steps"}
              {r.atMin != null
                ? ` · ${String(Math.floor(r.atMin / 60)).padStart(2, "0")}:${String(r.atMin % 60).padStart(2, "0")}`
                : ""}
            </span>
          </button>
        ))}
      </div>
      <div className="desk-routines__foot">
        <IconButton
          icon={Settings2}
          label="Edit routines"
          size="sm"
          onClick={() => void ipc.openSettingsWindow("desk")}
        />
      </div>
    </div>
  );
}
