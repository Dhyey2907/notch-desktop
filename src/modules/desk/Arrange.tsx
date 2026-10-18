// Desk → Arrange: snap windows into a layout (pick a window for each zone, then Arrange), and
// workspaces (save where every window is, put it all back later — starting apps that are closed).
import { RotateCcw, Save, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";

import type { Workspace } from "../../core/generated/Workspace";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, IconButton } from "../../design/components";
import { usePolled } from "../devtools/usePolled";
import { defaultPicks, LAYOUTS } from "./layouts";

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "That didn't work.";
}

function ago(seconds: number): string {
  const s = Math.max(0, Date.now() / 1000 - seconds);
  if (s < 90) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  return new Date(seconds * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

export function Arrange() {
  const windows = usePolled(() => ipc.deskWindows(), 4000);
  const [layoutId, setLayoutId] = useState(LAYOUTS[0].id);
  const layout = LAYOUTS.find((l) => l.id === layoutId) ?? LAYOUTS[0];
  // Your choices per layout; until you choose, the newest windows fill the zones.
  const [chosen, setChosen] = useState<Record<string, (number | null)[]>>({});
  const [status, setStatus] = useState<string | null>(null);
  const picks =
    chosen[layoutId] ??
    defaultPicks(
      (windows ?? []).map((w) => w.id),
      layout.zones.length,
    );
  const setPick = (zone: number, id: number | null) =>
    setChosen((c) => ({ ...c, [layoutId]: picks.map((v, j) => (j === zone ? id : v)) }));

  const arrange = async () => {
    let done = 0;
    for (let i = layout.zones.length - 1; i >= 0; i--) {
      const id = picks[i];
      if (id == null) continue;
      try {
        await ipc.deskSnap(id, layout.zones[i]);
        done += 1;
      } catch (e) {
        setStatus(message(e));
        return;
      }
    }
    setStatus(
      done ? `Arranged ${done} ${done === 1 ? "window" : "windows"}` : "Pick a window for a zone first",
    );
  };

  return (
    <div className="arrange">
      <div className="arrange__layouts" role="radiogroup" aria-label="Layout">
        {LAYOUTS.map((l) => (
          <button
            key={l.id}
            type="button"
            role="radio"
            aria-checked={l.id === layoutId}
            className={`arrange__layout${l.id === layoutId ? " arrange__layout--on" : ""}`}
            title={l.label}
            onClick={() => setLayoutId(l.id)}
          >
            <span className="arrange__mini" aria-hidden>
              {l.zones.map((z, i) => (
                <span
                  key={i}
                  style={{
                    left: `${z.x * 100}%`,
                    top: `${z.y * 100}%`,
                    width: `${z.w * 100}%`,
                    height: `${z.h * 100}%`,
                  }}
                />
              ))}
            </span>
            <span className="arrange__label">{l.label}</span>
          </button>
        ))}
      </div>
      <div className="arrange__screen">
        {layout.zones.map((z, i) => (
          <label
            key={`${layoutId}-${i}`}
            className="arrange__zone"
            style={{
              left: `${z.x * 100}%`,
              top: `${z.y * 100}%`,
              width: `${z.w * 100}%`,
              height: `${z.h * 100}%`,
            }}
          >
            <select
              aria-label={`Window for zone ${i + 1}`}
              value={picks[i] ?? ""}
              onChange={(e) => setPick(i, e.target.value ? Number(e.target.value) : null)}
            >
              <option value="">Empty</option>
              {(windows ?? []).map((w) => (
                <option key={w.id} value={w.id}>
                  {w.app} · {w.title}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      <div className="arrange__row">
        <Button variant="primary" onClick={() => void arrange()}>
          Arrange
        </Button>
        {status && <span className="arrange__status">{status}</span>}
      </div>
      <Workspaces />
    </div>
  );
}

function Workspaces() {
  const [list, setList] = useState<Workspace[]>([]);
  const [name, setName] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = () =>
    ipc.workspaceList().then(
      (l) => setList(l),
      () => {},
    );
  useEffect(() => {
    void reload();
  }, []);

  const save = () => {
    const n = name.trim();
    if (!n) return;
    void ipc.workspaceSave(n).then(
      (count) => {
        setStatus(`Saved ${count} ${count === 1 ? "window" : "windows"} as “${n}”`);
        setName("");
        void reload();
      },
      (e) => setStatus(message(e)),
    );
  };

  const restore = (ws: Workspace) => {
    setBusy(true);
    setStatus(`Putting back “${ws.name}”…`);
    void ipc
      .workspaceRestore(ws.name)
      .then(
        ([placed, started]) =>
          setStatus(
            `Placed ${placed} ${placed === 1 ? "window" : "windows"}${started ? `, started ${started} ${started === 1 ? "app" : "apps"}` : ""}`,
          ),
        (e) => setStatus(message(e)),
      )
      .finally(() => setBusy(false));
  };

  return (
    <section className="workspaces" aria-label="Workspaces">
      <form
        className="workspaces__save"
        onSubmit={(e) => {
          e.preventDefault();
          save();
        }}
      >
        <input
          className="workspaces__name"
          aria-label="Workspace name"
          placeholder="Save every window's place as… (e.g. Work)"
          value={name}
          maxLength={60}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => e.stopPropagation()}
        />
        <Button type="submit" disabled={!name.trim()}>
          <Save size={12} aria-hidden /> Save
        </Button>
      </form>
      {status && <p className="arrange__status">{status}</p>}
      {list.length > 0 && (
        <ul className="workspaces__list">
          {list.map((ws) => (
            <li key={ws.name} className="workspaces__item">
              <span className="workspaces__title">{ws.name}</span>
              <span className="workspaces__meta">
                {ws.windows.length} windows · {ago(ws.savedAt)}
              </span>
              <Button disabled={busy} onClick={() => restore(ws)}>
                <RotateCcw size={12} aria-hidden /> Restore
              </Button>
              <IconButton
                icon={Trash2}
                label={`Delete ${ws.name}`}
                size="sm"
                onClick={() => void ipc.workspaceDelete(ws.name).then(reload, () => {})}
              />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
