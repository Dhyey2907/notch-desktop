import { AppWindow, ChevronDown, ChevronUp, Folder, Globe, Pencil, Timer, X, XCircle } from "lucide-react";
import { useState } from "react";

import type { Routine } from "../core/generated/Routine";
import type { RoutineStep } from "../core/generated/RoutineStep";
import { ipc } from "../core/ipc";
import { Button, IconButton, Surface, Switch, TextField } from "../design/components";
import { minutesToTime, timeToMinutes } from "../shell/behavior";
import { useSettings } from ".";
import { Row, TimeField } from "./controls";
import { patch } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";
import { stepLabel } from "./routines";

const DAYS = ["M", "T", "W", "T", "F", "S", "S"];
const DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

const STEP_ICONS = { app: AppWindow, url: Globe, folder: Folder, close: XCircle, focus: Timer } as const;

function newId() {
  return `r${Math.random().toString(36).slice(2, 10)}`;
}

function Editor({
  routine,
  draft,
  onDone,
}: {
  routine: Routine | null;
  /** Starting point for a new routine (a template). */
  draft?: Routine;
  onDone: () => void;
}) {
  const start = routine ?? draft ?? null;
  const routines = useSettings((s) => s.settings?.desk.routines ?? []);
  const [name, setName] = useState(start?.name ?? "");
  const [steps, setSteps] = useState<RoutineStep[]>(start?.steps ?? []);
  const [url, setUrl] = useState("");
  const [exe, setExe] = useState("");
  const [scheduled, setScheduled] = useState(start?.atMin != null);
  const [atMin, setAtMin] = useState(start?.atMin ?? 9 * 60);
  const [days, setDays] = useState<number[]>(start?.days.length ? start.days : [1, 2, 3, 4, 5]);
  const [error, setError] = useState<string | null>(null);

  const add = (step: RoutineStep) => setSteps([...steps, step]);
  const move = (i: number, by: number) => {
    const next = [...steps];
    const [s] = next.splice(i, 1);
    next.splice(i + by, 0, s);
    setSteps(next);
  };

  const save = () => {
    const saved: Routine = {
      id: routine?.id ?? newId(),
      name: name.trim(),
      steps,
      atMin: scheduled ? atMin : undefined,
      days: scheduled ? days : [],
    };
    const next = routine ? routines.map((r) => (r.id === routine.id ? saved : r)) : [...routines, saved];
    void useSettings
      .getState()
      .update((s) => ({ ...s, desk: { ...s.desk, routines: next } }))
      .then(onDone, () => setError("Couldn't save — check that every step is complete."));
  };

  return (
    <Surface>
      <TextField label="Name" value={name} onChange={setName} placeholder="Start work" />
      <p className="settings__row-label settings__surface-title">Steps</p>
      {steps.length === 0 && <p className="settings__note">Add what the routine should do, in order.</p>}
      <ol className="routine-steps">
        {steps.map((step, i) => {
          const Icon = STEP_ICONS[step.kind];
          return (
            <li key={i} className="snippets__item">
              <Icon size={14} aria-hidden />
              <span className="snippets__body">{stepLabel(step)}</span>
              <IconButton
                icon={ChevronUp}
                label="Move up"
                size="sm"
                disabled={i === 0}
                onClick={() => move(i, -1)}
              />
              <IconButton
                icon={ChevronDown}
                label="Move down"
                size="sm"
                disabled={i === steps.length - 1}
                onClick={() => move(i, 1)}
              />
              <IconButton
                icon={X}
                label="Remove step"
                size="sm"
                onClick={() => setSteps(steps.filter((_, j) => j !== i))}
              />
            </li>
          );
        })}
      </ol>
      <div className="routine-add">
        <Button onClick={() => void ipc.routinePickApp().then((path) => path && add({ kind: "app", path }))}>
          <AppWindow size={13} aria-hidden /> Open an app…
        </Button>
        <Button
          onClick={() => void ipc.routinePickFolder().then((path) => path && add({ kind: "folder", path }))}
        >
          <Folder size={13} aria-hidden /> Open a folder…
        </Button>
        <Button onClick={() => add({ kind: "focus", minutes: 50 })}>
          <Timer size={13} aria-hidden /> Start focus (50 min)
        </Button>
      </div>
      <form
        className="routine-inline"
        onSubmit={(e) => {
          e.preventDefault();
          const u = url.trim();
          if (!u) return;
          add({ kind: "url", url: /^https?:\/\//.test(u) ? u : `https://${u}` });
          setUrl("");
        }}
      >
        <TextField label="Open a website" value={url} onChange={setUrl} placeholder="mail.google.com" />
        <Button type="submit" disabled={!url.trim()}>
          Add
        </Button>
      </form>
      <form
        className="routine-inline"
        onSubmit={(e) => {
          e.preventDefault();
          const x = exe.trim();
          if (!x) return;
          add({ kind: "close", exe: /\.exe$/i.test(x) ? x : `${x}.exe` });
          setExe("");
        }}
      >
        <TextField
          label="Close an app"
          value={exe}
          onChange={setExe}
          placeholder="Discord.exe"
          description="Closes it like clicking × — apps can still ask to save."
        />
        <Button type="submit" disabled={!exe.trim()}>
          Add
        </Button>
      </form>
      <Switch
        label="Run by itself"
        description="At a set time on the days you pick."
        checked={scheduled}
        onChange={setScheduled}
      />
      {scheduled && (
        <Row label="When">
          <span className="settings-inline">
            <TimeField
              label="Run at"
              value={minutesToTime(atMin)}
              onChange={(t) => {
                const m = timeToMinutes(t);
                if (m !== null) setAtMin(m);
              }}
            />
            <span className="settings-days" role="group" aria-label="Days">
              {DAYS.map((letter, i) => (
                <button
                  key={i}
                  type="button"
                  aria-pressed={days.includes(i + 1)}
                  aria-label={DAY_NAMES[i]}
                  className={`settings-days__day${days.includes(i + 1) ? " settings-days__day--on" : ""}`}
                  onClick={() =>
                    setDays(days.includes(i + 1) ? days.filter((d) => d !== i + 1) : [...days, i + 1].sort())
                  }
                >
                  {letter}
                </button>
              ))}
            </span>
          </span>
        </Row>
      )}
      {error && <p className="settings__note settings__note--warning">{error}</p>}
      <div className="settings__buttons settings__buttons--start">
        <Button variant="primary" onClick={save} disabled={!name.trim() || steps.length === 0}>
          {routine ? "Save routine" : "Add routine"}
        </Button>
        <Button onClick={onDone}>Cancel</Button>
      </div>
    </Surface>
  );
}

/** Ready-made starting points; apps are added with "Open an app…". */
const TEMPLATES: { name: string; description: string; routine: Routine }[] = [
  {
    name: "Start work",
    description: "Mail and calendar open, then 50 minutes of focus.",
    routine: {
      id: "",
      name: "Start work",
      steps: [
        { kind: "url", url: "https://mail.google.com" },
        { kind: "url", url: "https://calendar.google.com" },
        { kind: "focus", minutes: 50 },
      ],
      atMin: 9 * 60,
      days: [1, 2, 3, 4, 5],
    },
  },
  {
    name: "Gaming",
    description: "Closes chat and work apps so nothing pops up.",
    routine: {
      id: "",
      name: "Gaming",
      steps: [
        { kind: "close", exe: "Teams.exe" },
        { kind: "close", exe: "Slack.exe" },
      ],
      days: [],
    },
  },
  {
    name: "Wind down",
    description: "Closes your editor and starts a short focus to wrap up.",
    routine: {
      id: "",
      name: "Wind down",
      steps: [
        { kind: "close", exe: "Code.exe" },
        { kind: "focus", minutes: 15 },
      ],
      days: [],
    },
  },
];

export function DeskSection() {
  const routines = useSettings((s) => s.settings?.desk.routines ?? []);
  const [editing, setEditing] = useState<Routine | "new" | null>(null);
  const [draft, setDraft] = useState<Routine | undefined>(undefined);
  const startNew = (from?: Routine) => {
    setDraft(from);
    setEditing("new");
  };

  return (
    <Section title="Desk" lead={pageLead("desk")}>
      {editing ? (
        <Editor routine={editing === "new" ? null : editing} draft={draft} onDone={() => setEditing(null)} />
      ) : routines.length === 0 ? (
        <Surface>
          <div id="setting-routines" className="desk-empty">
            <p className="settings__row-label">No routines yet</p>
            <p className="settings__row-description">
              A routine sets up your desk in one click from Desk → Routines — open apps, sites and folders,
              close what you don't need, start focus. It can also run by itself at a set time.
            </p>
          </div>
          <div className="desk-templates">
            {TEMPLATES.map((t) => (
              <button
                key={t.name}
                type="button"
                className="desk-template"
                onClick={() => startNew(t.routine)}
              >
                <span className="desk-template__name">{t.name}</span>
                <span className="desk-template__description">{t.description}</span>
              </button>
            ))}
            <button type="button" className="desk-template desk-template--blank" onClick={() => startNew()}>
              <span className="desk-template__name">+ Blank routine</span>
              <span className="desk-template__description">Start from nothing.</span>
            </button>
          </div>
        </Surface>
      ) : (
        <Surface>
          <ul id="setting-routines" className="snippets" aria-label="Routines">
            {routines.map((r) => (
              <li key={r.id} className="snippets__item">
                <kbd className="settings__kbd">{r.name}</kbd>
                <span className="snippets__body">
                  {r.steps.map(stepLabel).join(" → ")}
                  {r.atMin != null ? ` · ${minutesToTime(r.atMin)}` : ""}
                </span>
                <IconButton icon={Pencil} label={`Edit ${r.name}`} size="sm" onClick={() => setEditing(r)} />
                <IconButton
                  icon={X}
                  label={`Delete ${r.name}`}
                  size="sm"
                  onClick={() => patch("desk", { routines: routines.filter((x) => x.id !== r.id) })}
                />
              </li>
            ))}
          </ul>
          <div className="settings__buttons settings__buttons--start">
            <Button variant="primary" onClick={() => startNew()}>
              New routine
            </Button>
          </div>
        </Surface>
      )}
      <Surface>
        <Row
          label="Window tricks"
          description="Keep any window on top (Ctrl + Alt + P, or change it in Shortcuts & gestures). In Desk → Windows you can also make a window see-through or watch it live in the notch."
        >
          <span />
        </Row>
      </Surface>
    </Section>
  );
}
