// Quick capture (Ctrl+Alt+Space): type one line, see where it'll go, press Enter.
// Read on this PC; "Let AI sort it" asks the AI chosen in Settings → AI instead.
import { CornerDownLeft, Sparkles } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { CaptureKind } from "../../core/generated/CaptureKind";
import { ipc, isIpcError } from "../../core/ipc";
import { IconButton, ModuleContainer } from "../../design/components";
import { useSettings } from "../../settings";
import { reminderLabel, remindChoices } from "../todo/remind";
import { KIND_LABELS, parseCapture, type Capture } from "./parse";
import "./capture.css";

const KINDS: CaptureKind[] = ["task", "reminder", "note", "event", "link"];

interface Filed {
  id: number;
  text: string;
  where: string;
}

function message(error: unknown) {
  return isIpcError(error) ? error.message.replace(/^invalid input: /, "") : "That didn't work.";
}

const WHERE: Record<CaptureKind, string> = {
  task: "To-do",
  reminder: "To-do · reminder",
  note: "Notes",
  event: "Google Calendar",
  link: "Read later",
};

export default function CaptureExpanded() {
  const canEdit = useSettings((s) => s.settings?.calendar.edit ?? false);
  const [text, setText] = useState("");
  // A kind the user picked, or the AI's whole guess; otherwise the local reading.
  const [pick, setPick] = useState<CaptureKind | null>(null);
  const [aiGuess, setAiGuess] = useState<{ text: string; capture: Capture } | null>(null);
  const [sorting, setSorting] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filed, setFiled] = useState<Filed[]>([]);
  const input = useRef<HTMLInputElement>(null);
  const now = new Date();

  useEffect(() => {
    input.current?.focus();
  }, []);

  const local = parseCapture(text, now);
  const base = aiGuess?.text === text ? aiGuess.capture : local;
  const picked: Capture = pick ? { ...base, kind: pick } : base;
  const capture: Capture =
    pick === "link" && !picked.url ? { ...picked, url: /https?:\/\/\S+/i.exec(text)?.[0] } : picked;
  const needsTime = capture.kind === "reminder" && !capture.at;

  const reset = () => {
    setText("");
    setPick(null);
    setAiGuess(null);
    setError(null);
    input.current?.focus();
  };

  const file = async (c: Capture) => {
    const title = c.title.trim() || text.trim();
    if (!title) return;
    setBusy(true);
    setError(null);
    try {
      let where = WHERE[c.kind];
      switch (c.kind) {
        case "task":
          await ipc.todoAddTask(title, c.minutes ?? null, true);
          break;
        case "reminder":
          if (!c.at) throw new Error("pick a time");
          await ipc.todoAddReminder(title, c.at);
          where = `To-do · ${reminderLabel(c.at, new Date())}`;
          break;
        case "note":
          await ipc.noteAdd(title);
          break;
        case "event":
          if (c.at && aiGuess?.text === text) {
            await ipc.calendarCreateEvent({
              title,
              start: new Date(c.at).toISOString(),
              end: new Date(c.at + 30 * 60_000).toISOString(),
              allDay: false,
            });
          } else {
            // Google reads the sentence itself ("lunch with Ann friday 1pm").
            const made = await ipc.calendarQuickAdd(
              text.replace(/^(?:event|meeting|cal|calendar)\s*[:-]\s*/i, ""),
            );
            if (made.start) where = `Calendar · ${reminderLabel(Date.parse(made.start), new Date())}`;
          }
          break;
        case "link":
          if (!c.url) throw new Error("no link");
          await ipc.readLaterAdd(c.url);
          break;
      }
      setFiled((f) => [{ id: Date.now(), text: title, where }, ...f].slice(0, 5));
      reset();
    } catch (err) {
      setError(
        err instanceof Error && err.message === "pick a time"
          ? "Pick when to be reminded"
          : err instanceof Error && err.message === "no link"
            ? "There's no link in that"
            : message(err),
      );
    } finally {
      setBusy(false);
    }
  };

  const sort = async () => {
    if (!text.trim()) return;
    setSorting(true);
    setError(null);
    try {
      const guess = await ipc.captureSort(text);
      const url = /https?:\/\/\S+/i.exec(text)?.[0];
      setAiGuess({
        text,
        capture: {
          kind: guess.kind,
          title: guess.title,
          at: guess.at ? Date.parse(guess.at) : undefined,
          minutes: guess.minutes,
          url: guess.kind === "link" ? url : undefined,
        },
      });
      setPick(null);
    } catch (err) {
      setError(message(err));
    } finally {
      setSorting(false);
    }
  };

  const preview = !text.trim()
    ? "Try “call mom at 5”, “note: …”, “lunch with Ann friday 1pm”, a link, or “tea 4m”."
    : [
        `→ ${WHERE[capture.kind]}`,
        capture.title && capture.kind !== "link" ? `“${capture.title}”` : capture.url,
        capture.at ? reminderLabel(capture.at, now) : null,
        capture.minutes ? `${capture.minutes} min timer` : null,
        aiGuess?.text === text && !pick ? "sorted by AI" : null,
      ]
        .filter(Boolean)
        .join(" · ");

  return (
    <ModuleContainer title="Quick capture">
      <div className="capture">
        <form
          className="capture__bar"
          onSubmit={(e) => {
            e.preventDefault();
            if (!needsTime) void file(capture);
          }}
        >
          <input
            ref={input}
            className="capture__input"
            aria-label="Capture"
            placeholder="What's on your mind?"
            value={text}
            maxLength={500}
            disabled={busy}
            onChange={(e) => {
              setText(e.target.value);
              setError(null);
            }}
          />
          <IconButton
            icon={Sparkles}
            label={sorting ? "Sorting…" : "Let AI sort it"}
            size="sm"
            disabled={!text.trim() || sorting || busy}
            onClick={() => void sort()}
          />
          <IconButton
            icon={CornerDownLeft}
            label="File it"
            size="sm"
            disabled={!text.trim() || busy || needsTime}
            onClick={() => void file(capture)}
          />
        </form>
        <p className={`capture__preview${error ? " capture__preview--error" : ""}`} aria-live="polite">
          {sorting ? "Asking the AI…" : (error ?? preview)}
        </p>
        {text.trim() && (
          <div className="capture__kinds" role="radiogroup" aria-label="File as">
            {KINDS.map((k) => (
              <button
                key={k}
                type="button"
                role="radio"
                aria-checked={capture.kind === k}
                className={`nd-chip${capture.kind === k ? " nd-chip--on" : ""}`}
                onClick={() => setPick(k)}
              >
                {KIND_LABELS[k]}
              </button>
            ))}
          </div>
        )}
        {capture.kind === "event" && text.trim() && !canEdit && (
          <p className="capture__hint">
            Adding events needs “Add events and answer invites” in Settings → Calendar.{" "}
            <button
              type="button"
              className="capture__link"
              onClick={() => void ipc.openSettingsWindow("calendar")}
            >
              Open settings
            </button>
          </p>
        )}
        {needsTime && text.trim() && (
          <div className="capture__kinds" role="group" aria-label="Remind me">
            {remindChoices(now).map((c) => (
              <button
                key={c.label}
                type="button"
                className="nd-chip"
                onClick={() => void file({ ...capture, at: c.at })}
              >
                {c.label}
              </button>
            ))}
          </div>
        )}
        {filed.length > 0 && (
          <ul className="capture__filed" aria-label="Just filed">
            {filed.map((f) => (
              <li key={f.id}>
                <span className="capture__filed-text">{f.text}</span>
                <span className="capture__filed-where">{f.where}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </ModuleContainer>
  );
}
