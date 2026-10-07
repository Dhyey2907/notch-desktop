// Tasks | Notes | Focus. Tasks can carry a timer ("Write report 25m") or a reminder at a set
// time; the bell decides whether it rings. Notes save as you type.
import { AlarmClock, Bell, BellOff, Check, Plus, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent } from "react";

import type { Note } from "../../core/generated/Note";
import type { Task } from "../../core/generated/Task";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer, SegmentedControl } from "../../design/components";
import { FocusView } from "./FocusView";
import { formatCountdown, formatMinutes, parseQuickAdd } from "./parse";
import { remindChoices, reminderLabel, toLocalInput } from "./remind";
import { useSecondClock } from "./useSecondClock";
import { runningTimers, useTodo } from "./todoStore";
import "./todo.css";

type Tab = "tasks" | "notes" | "focus";
const TIMER_CHOICES = [5, 15, 25, 60];

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

export default function TodoExpanded() {
  const [tab, setTab] = useState<Tab>("tasks");
  return (
    <ModuleContainer
      title="To-do"
      trailing={
        <SegmentedControl<Tab>
          label="To-do view"
          value={tab}
          options={[
            { value: "tasks", label: "Tasks" },
            { value: "notes", label: "Notes" },
            { value: "focus", label: "Focus" },
          ]}
          onChange={setTab}
        />
      }
    >
      {tab === "tasks" ? <Tasks /> : tab === "notes" ? <Notes /> : <FocusView />}
    </ModuleContainer>
  );
}

function RingToggle({ ring, onChange }: { ring: boolean; onChange: (ring: boolean) => void }) {
  return (
    <button
      type="button"
      className={`todo-bell${ring ? " todo-bell--on" : ""}`}
      aria-pressed={ring}
      aria-label={ring ? "Rings when it's time (on)" : "Rings when it's time (off)"}
      title={ring ? "Rings when it's time" : "Silent"}
      onClick={() => onChange(!ring)}
    >
      {ring ? <Bell size={14} aria-hidden /> : <BellOff size={14} aria-hidden />}
    </button>
  );
}

/** Pick a timer length or a time to be reminded. */
function RemindPicker({
  ring,
  onRing,
  onTimer,
  onAt,
}: {
  ring: boolean;
  onRing: (ring: boolean) => void;
  onTimer?: (minutes: number) => void;
  onAt: (atMs: number) => void;
}) {
  const [now] = useState(() => new Date());
  const [custom, setCustom] = useState(() => toLocalInput(now.getTime() + 2 * 3600_000));
  return (
    <div className="todo-remind" role="group" aria-label="Remind me">
      {onTimer && (
        <div className="todo-remind__row">
          <span className="todo-remind__label">Timer</span>
          {TIMER_CHOICES.map((minutes) => (
            <button key={minutes} type="button" className="todo-chip" onClick={() => onTimer(minutes)}>
              {formatMinutes(minutes)}
            </button>
          ))}
        </div>
      )}
      <div className="todo-remind__row">
        <span className="todo-remind__label">Remind me</span>
        {remindChoices(now).map((c) => (
          <button
            key={c.label}
            type="button"
            className="todo-chip"
            title={reminderLabel(c.at, now)}
            onClick={() => onAt(c.at)}
          >
            {c.label}
          </button>
        ))}
      </div>
      <div className="todo-remind__row">
        <span className="todo-remind__label">At</span>
        <input
          type="datetime-local"
          className="todo-remind__date"
          aria-label="Pick a date and time"
          value={custom}
          min={toLocalInput(now.getTime())}
          onChange={(e) => setCustom(e.target.value)}
        />
        <button
          type="button"
          className="todo-chip todo-chip--primary"
          disabled={!custom}
          onClick={() => {
            const at = new Date(custom).getTime();
            if (!Number.isNaN(at)) onAt(at);
          }}
        >
          Set
        </button>
        <RingToggle ring={ring} onChange={onRing} />
      </div>
    </div>
  );
}

function Tasks() {
  const snapshot = useTodo((s) => s.snapshot);
  const [text, setText] = useState("");
  const [ring, setRing] = useState(true);
  const [picking, setPicking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const tasks = snapshot?.tasks ?? [];
  const now = useSecondClock(runningTimers(tasks).length > 0);
  const open = tasks.filter((t) => !t.done);
  const done = tasks.filter((t) => t.done);
  const parsed = parseQuickAdd(text);

  const add = async () => {
    if (!parsed.title) return;
    try {
      await ipc.todoAddTask(parsed.title, parsed.minutes ?? null, ring);
      setText("");
      setError(null);
    } catch (err) {
      setError(message(err));
    }
  };

  const addWithReminder = async (atMs: number) => {
    if (!parsed.title) {
      setError("Type the task first, then pick when to be reminded");
      return;
    }
    try {
      const task = await ipc.todoAddReminder(parsed.title, atMs);
      if (!ring) await ipc.todoRemindAt(task.id, atMs, false);
      setText("");
      setPicking(false);
      setError(null);
    } catch (err) {
      setError(message(err));
    }
  };

  return (
    <div className="todo">
      <form
        className="todo-add"
        onSubmit={(e: FormEvent) => {
          e.preventDefault();
          void add();
        }}
      >
        <input
          className="todo-add__input"
          aria-label="New task"
          placeholder="Add a task — e.g. “Write report 25m”"
          value={text}
          maxLength={200}
          onChange={(e) => setText(e.target.value)}
        />
        <button
          type="button"
          className={`todo-bell${picking ? " todo-bell--on" : ""}`}
          aria-expanded={picking}
          aria-label="Remind me about this task"
          title="Remind me…"
          onClick={() => setPicking(!picking)}
        >
          <AlarmClock size={14} aria-hidden />
        </button>
        <IconButton
          icon={Plus}
          label="Add task"
          size="sm"
          disabled={!parsed.title}
          onClick={() => void add()}
        />
      </form>
      {picking && <RemindPicker ring={ring} onRing={setRing} onAt={(at) => void addWithReminder(at)} />}
      <p className="todo-hint">
        {error ??
          (parsed.minutes
            ? `Starts a ${formatMinutes(parsed.minutes)} timer${ring ? " that rings" : " (silent)"}`
            : " ")}
      </p>

      {tasks.length === 0 ? (
        <EmptyState
          title="Nothing to do"
          body="Add a task above. Put a time at the end for a timer, or use the alarm button to pick when to be reminded."
        />
      ) : (
        <ul className="todo-list" aria-label="Tasks">
          {open.map((task) => (
            <TaskRow key={task.id} task={task} now={now} onError={setError} />
          ))}
          {done.length > 0 && (
            <li className="todo-list__section">
              <span>Done · {done.length}</span>
              <Button onClick={() => void ipc.todoClearDone().catch((err) => setError(message(err)))}>
                Clear
              </Button>
            </li>
          )}
          {done.map((task) => (
            <TaskRow key={task.id} task={task} now={now} onError={setError} />
          ))}
        </ul>
      )}
    </div>
  );
}

function TaskRow({ task, now, onError }: { task: Task; now: number; onError: (message: string) => void }) {
  const [picking, setPicking] = useState(false);
  const [ring, setRing] = useState(task.ring);
  const run = (action: Promise<void>) => void action.catch((err) => onError(message(err)));
  const endsAt = task.timerEndsAt ? new Date(task.timerEndsAt).getTime() : null;
  // A reminder (a set time) shows when; a timer counts down.
  const isReminder = endsAt !== null && task.timerSecs == null;

  return (
    <li className={`todo-task${task.done ? " todo-task--done" : ""}`}>
      <div className="todo-task__row">
        <button
          type="button"
          className={`todo-check${task.done ? " todo-check--done" : ""}`}
          role="checkbox"
          aria-checked={task.done}
          aria-label={task.done ? `Mark “${task.title}” not done` : `Mark “${task.title}” done`}
          onClick={() => run(ipc.todoSetDone(task.id, !task.done))}
        >
          {task.done && <Check size={12} strokeWidth={3} aria-hidden />}
        </button>
        <span className="todo-task__title" title={task.title}>
          {task.title}
        </span>
        {endsAt !== null && !task.done ? (
          <span className="todo-task__timer" title={isReminder ? "Reminder" : "Timer"}>
            {task.ring ? <Bell size={11} aria-label="Rings" /> : <BellOff size={11} aria-label="Silent" />}
            <span className="todo-task__countdown">
              {isReminder ? reminderLabel(endsAt, new Date(now)) : formatCountdown(endsAt - now)}
            </span>
            <IconButton
              icon={X}
              label={isReminder ? "Cancel reminder" : "Stop timer"}
              size="sm"
              onClick={() => run(ipc.todoCancelTimer(task.id))}
            />
          </span>
        ) : (
          !task.done && (
            <button
              type="button"
              className={`todo-bell${picking ? " todo-bell--on" : ""}`}
              aria-expanded={picking}
              aria-label="Remind me or start a timer"
              title="Remind me…"
              onClick={() => setPicking(!picking)}
            >
              <AlarmClock size={14} aria-hidden />
            </button>
          )
        )}
        <IconButton
          icon={Trash2}
          label={`Delete “${task.title}”`}
          size="sm"
          onClick={() => run(ipc.todoDelete(task.id))}
        />
      </div>
      {picking && endsAt === null && (
        <RemindPicker
          ring={ring}
          onRing={setRing}
          onTimer={(minutes) => {
            setPicking(false);
            run(ipc.todoStartTimer(task.id, minutes, ring));
          }}
          onAt={(at) => {
            setPicking(false);
            run(ipc.todoRemindAt(task.id, at, ring));
          }}
        />
      )}
    </li>
  );
}

function Notes() {
  const notes = useTodo((s) => s.snapshot?.notes ?? []);
  const [error, setError] = useState<string | null>(null);
  const [focusId, setFocusId] = useState<string | null>(null);

  const add = async () => {
    try {
      const note = await ipc.noteAdd("");
      setFocusId(note.id);
      setError(null);
    } catch (err) {
      setError(message(err));
    }
  };

  return (
    <div className="todo">
      <div className="todo-notes__bar">
        <Button onClick={() => void add()}>New note</Button>
        {error && <span className="todo-hint todo-hint--error">{error}</span>}
      </div>
      {notes.length === 0 ? (
        <EmptyState title="No notes yet" body="Jot things down here — they save as you type." />
      ) : (
        <ul className="todo-list" aria-label="Notes">
          {notes.map((note) => (
            <NoteRow key={note.id} note={note} autoFocus={note.id === focusId} onError={setError} />
          ))}
        </ul>
      )}
    </div>
  );
}

function NoteRow({
  note,
  autoFocus,
  onError,
}: {
  note: Note;
  autoFocus: boolean;
  onError: (m: string) => void;
}) {
  // Edits stay local while typing and save after a short pause, so the list doesn't
  // reorder under the cursor on every keystroke.
  const [body, setBody] = useState(note.body);
  const timer = useRef<number | undefined>(undefined);
  const latest = useRef(body);

  const save = () => {
    window.clearTimeout(timer.current);
    if (latest.current !== note.body) {
      void ipc.noteUpdate(note.id, latest.current).catch((err) => onError(message(err)));
    }
  };

  useEffect(() => () => window.clearTimeout(timer.current), []);

  return (
    <li className="todo-note">
      <textarea
        className="todo-note__body"
        aria-label="Note"
        placeholder="Write something…"
        value={body}
        maxLength={10_000}
        rows={3}
        autoFocus={autoFocus}
        onChange={(e) => {
          setBody(e.target.value);
          latest.current = e.target.value;
          window.clearTimeout(timer.current);
          timer.current = window.setTimeout(save, 800);
        }}
        onBlur={save}
      />
      <IconButton
        icon={Trash2}
        label="Delete note"
        size="sm"
        onClick={() => void ipc.noteDelete(note.id).catch((err) => onError(message(err)))}
      />
    </li>
  );
}
