// Ask → My PC: a question about your own files, notes, mail and calendar. The AI picks what to
// search, Overline searches here, and the answer cites what it found (click a file to open it).
import { CalendarDays, CornerDownLeft, File, Folder, FolderOpen, Mail, StickyNote } from "lucide-react";
import { useState, type FormEvent } from "react";

import type { PcAnswer } from "../../core/generated/PcAnswer";
import { ipc, isIpcError } from "../../core/ipc";
import { EmptyState, IconButton } from "../../design/components";

const ICONS = { file: File, folder: Folder, note: StickyNote, mail: Mail, event: CalendarDays } as const;

export function AskPc() {
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<PcAnswer | null>(null);
  const [error, setError] = useState<string | null>(null);

  const send = (e?: FormEvent) => {
    e?.preventDefault();
    if (!question.trim() || busy) return;
    setBusy(true);
    setError(null);
    void ipc
      .askPc(question)
      .then(setResult, (err) =>
        setError(isIpcError(err) ? err.message.replace(/^invalid input: /, "") : "That didn't work"),
      )
      .finally(() => setBusy(false));
  };

  return (
    <div className="ask">
      <div className="ask__list">
        {busy ? (
          <p className="askpc__busy">
            Looking through your files, notes{result?.skippedGoogle === false ? ", mail" : ""}…
          </p>
        ) : result ? (
          <div className="askpc">
            <p className="askpc__answer">{result.answer}</p>
            {result.found.length > 0 && (
              <ol className="askpc__found">
                {result.found.map((f, i) => {
                  const Icon = ICONS[f.kind as keyof typeof ICONS] ?? File;
                  return (
                    <li key={i}>
                      <span className="askpc__n">{i + 1}</span>
                      <Icon size={13} aria-hidden />
                      {f.path ? (
                        <button
                          type="button"
                          className="askpc__open"
                          title={`Open ${f.path}`}
                          onClick={() => void ipc.filesOpen(f.path!).catch(() => {})}
                        >
                          {f.title}
                        </button>
                      ) : (
                        <span className="askpc__title">{f.title}</span>
                      )}
                      <span className="askpc__detail">{f.detail}</span>
                      {f.path && (
                        <IconButton
                          icon={FolderOpen}
                          label="Show in folder"
                          size="sm"
                          onClick={() => void ipc.filesReveal(f.path!).catch(() => {})}
                        />
                      )}
                    </li>
                  );
                })}
              </ol>
            )}
            {result.skippedGoogle && (
              <p className="askpc__note">
                Gmail and your calendar weren't searched: allow Google data for AI in Settings → Your day.
              </p>
            )}
          </div>
        ) : (
          <EmptyState
            title="Ask your PC"
            body="“The HDFC statement from last week”, “where did I save the resume?”, “what did Rahul mail about the invoice?” File names, folders and dates are searched here; file contents are never sent."
          />
        )}
      </div>
      {error && <p className="ask__error">{error}</p>}
      <form className="ask__form" onSubmit={send}>
        <textarea
          className="ask__input"
          aria-label="Question about your PC"
          placeholder={busy ? "Looking…" : "Ask about your files, notes, mail…"}
          rows={1}
          maxLength={500}
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) send(e);
          }}
        />
        <IconButton
          icon={CornerDownLeft}
          label="Ask"
          size="sm"
          disabled={!question.trim() || busy}
          onClick={() => send()}
        />
      </form>
    </div>
  );
}

/** Ask → Research: a topic researched in the background; a card says when the brief is ready. */
export function Research() {
  const [topic, setTopic] = useState("");
  const [sent, setSent] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  const start = (e?: FormEvent) => {
    e?.preventDefault();
    const t = topic.trim();
    if (!t) return;
    void ipc.researchStart(t).then(
      () => {
        setSent((s) => [t, ...s].slice(0, 5));
        setTopic("");
        setError(null);
      },
      (err) => setError(isIpcError(err) ? err.message.replace(/^invalid input: /, "") : "Couldn't start it"),
    );
  };

  return (
    <div className="ask">
      <div className="ask__list">
        {sent.length > 0 ? (
          <ul className="askpc__found">
            {sent.map((t, i) => (
              <li key={`${t}-${i}`}>
                <span className="askpc__title">Researching “{t}”</span>
                <span className="askpc__detail">a card will say when it's ready · saved to Notes</span>
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState
            title="Research in the background"
            body="Give a topic and keep working: the AI reads the web and a card brings back a short brief, saved to Notes with its sources."
          />
        )}
      </div>
      {error && <p className="ask__error">{error}</p>}
      <form className="ask__form" onSubmit={start}>
        <textarea
          className="ask__input"
          aria-label="Topic to research"
          placeholder="e.g. best budget 4K monitors 2026"
          rows={1}
          maxLength={300}
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) start(e);
          }}
        />
        <IconButton
          icon={CornerDownLeft}
          label="Research"
          size="sm"
          disabled={!topic.trim()}
          onClick={() => start()}
        />
      </form>
    </div>
  );
}
