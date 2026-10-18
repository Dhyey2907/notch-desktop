// Home → What did I miss?: the cards the notch showed in the last two days, newest first, by
// module. Private modules (messages, mail, clipboard…) show only their titles.
import { ArrowLeft, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

import type { HistoryEntry } from "../../core/generated/HistoryEntry";
import { ipc } from "../../core/ipc";
import { EmptyState, IconButton } from "../../design/components";
import { findModule } from "../registry";

function when(at: number, now: number): string {
  const d = new Date(at);
  const time = d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const today = new Date(now).toDateString() === d.toDateString();
  return today ? time : `Yesterday ${time}`;
}

export function HistoryView({ onBack }: { onBack: () => void }) {
  const [entries, setEntries] = useState<HistoryEntry[] | null>(null);
  const [module, setModule] = useState<string | null>(null);
  const [now] = useState(() => Date.now());

  useEffect(() => {
    ipc.historyList(48).then(setEntries, () => setEntries([]));
  }, []);

  const modules = useMemo(() => [...new Set((entries ?? []).map((e) => e.module))], [entries]);
  const shown = (entries ?? []).filter((e) => !module || e.module === module);

  return (
    <section className="home history" aria-label="What did I miss?">
      <header className="history__bar">
        <IconButton icon={ArrowLeft} label="Back" size="sm" onClick={onBack} />
        <span className="history__heading">What did I miss?</span>
        {entries && entries.length > 0 && (
          <IconButton
            icon={Trash2}
            label="Clear the history"
            size="sm"
            onClick={() => void ipc.historyClear().then(() => setEntries([]))}
          />
        )}
      </header>
      {modules.length > 1 && (
        <div className="history__filters" role="radiogroup" aria-label="Show">
          <button
            type="button"
            role="radio"
            aria-checked={!module}
            className="nd-chip"
            onClick={() => setModule(null)}
          >
            All
          </button>
          {modules.map((m) => (
            <button
              key={m}
              type="button"
              role="radio"
              aria-checked={module === m}
              className="nd-chip"
              onClick={() => setModule(m)}
            >
              {findModule(m)?.name ?? m}
            </button>
          ))}
        </div>
      )}
      {entries === null ? null : shown.length === 0 ? (
        <EmptyState title="Nothing yet" body="Cards the notch shows are listed here for two days." />
      ) : (
        <ul className="history__list">
          {shown.map((e, i) => {
            const Icon = findModule(e.module)?.icon;
            return (
              <li
                key={`${e.at}-${i}`}
                className={`history__item${e.tone ? ` history__item--${e.tone}` : ""}`}
              >
                <span className="history__time">{when(e.at, now)}</span>
                {Icon && <Icon size={13} aria-hidden />}
                <span className="history__text">
                  <span className="history__title">{e.title}</span>
                  {e.body && <span className="history__body">{e.body}</span>}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
