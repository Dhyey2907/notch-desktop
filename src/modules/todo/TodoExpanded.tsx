// Tasks | Notes. Tasks can carry a timer ("Write report 25m"); the bell decides whether the
// timer rings when it ends. Notes save as you type.
import { Bell, BellOff, Check, Circle, Plus, Timer, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState, type FormEvent } from "react";

import type { Note } from "../../core/generated/Note";
import type { Task } from "../../core/generated/Task";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer, SegmentedControl } from "../../design/components";
import { FocusView } from "./FocusView";
import { formatCountdown, formatMinutes, parseQuickAdd } from "./parse";
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
    <IconButton
      icon={ring ? Bell : BellOff}
      label={ring ? "Ring when the timer ends (on)" : "Ring when the timer ends (off)"}
      active={ring}
      size="sm"
      onClick={() => onChange(!ring)}
    />
  );
}

function Tasks() {
  const snapshot = useTodo((s) => s.snapshot);
  const [text, setText] = useState("");
  const [ring, setRing] = useState(true);
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
          placeholder="Add a task — add a time for a timer, e.g. “Write report 25m”"
          value={text}
          maxLength={200}
          onChange={(e) => setText(e.target.value)}
        />
        <RingToggle ring={ring} onChange={setRing} />
        <IconButton
          icon={Plus}
          label="Add task"
          size="sm"
          disabled={!parsed.title}
          onClick={() => void add()}
        />
      </form>
      <p className="todo-hint">
        {error ??
          (parsed.minutes
            ? `Starts a ${formatMinutes(parsed.minutes)} timer${ring ? " that rings" : " (silent)"}`
            : " ")}
      </p>

      {tasks.length === 0 ? (
        <EmptyState title="Nothing to do" body="Add a task above. Put a time at the end to start a timer." />
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

  return (
    <li className={`todo-task${task.done ? " todo-task--done" : ""}`}>
      <div className="todo-task__row">
        <IconButton
          icon={task.done ? Check : Circle}
          label={task.done ? `Mark “${task.title}” not done` : `Mark “${task.title}” done`}
          size="sm"
          active={task.done}
          onClick={() => run(ipc.todoSetDone(task.id, !task.done))}
        />
        <span className="todo-task__title" title={task.title}>
          {task.title}
        </span>
        {endsAt !== null ? (
          <span className="todo-task__timer">
            {task.ring ? <Bell size={11} aria-label="Rings" /> : <BellOff size={11} aria-label="Silent" />}
            <span className="todo-task__countdown">{formatCountdown(endsAt - now)}</span>
            <IconButton
              icon={X}
              label="Stop timer"
              size="sm"
              onClick={() => run(ipc.todoCancelTimer(task.id))}
            />
          </span>
        ) : (
          !task.done && (
            <IconButton
              icon={Timer}
              label="Start a timer"
              size="sm"
              active={picking}
              onClick={() => setPicking(!picking)}
            />
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
        <div className="todo-task__picker" role="group" aria-label="Timer length">
          {TIMER_CHOICES.map((minutes) => (
            <button
              key={minutes}
              type="button"
              className="todo-chip"
              onClick={() => {
                setPicking(false);
                run(ipc.todoStartTimer(task.id, minutes, ring));
              }}
            >
              {formatMinutes(minutes)}
            </button>
          ))}
          <RingToggle ring={ring} onChange={setRing} />
        </div>
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
