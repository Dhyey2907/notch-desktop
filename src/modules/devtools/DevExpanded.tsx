// Developer: Servers | Time | Run | Widgets.
import { Copy, ExternalLink, Flame, Play, Square } from "lucide-react";
import { useState } from "react";

import type { Language } from "../../core/generated/Language";
import type { RunResult } from "../../core/generated/RunResult";
import { ipc, isIpcError } from "../../core/ipc";
import {
  Button,
  ConfirmButton,
  EmptyState,
  IconButton,
  ModuleContainer,
  SegmentedControl,
} from "../../design/components";
import { hoursMinutes, useDevTools, type DevTab } from "./devStore";
import { usePolled } from "./usePolled";
// The list rows share the Clipboard module's look (styles only).
import "../clipboard/clipboard.css";
import "./dev.css";

function message(error: unknown) {
  return isIpcError(error)
    ? error.message.replace(/^invalid input: /, "")
    : "Something went wrong. Try again.";
}

export default function DevExpanded() {
  const tab = useDevTools((s) => s.tab);
  return (
    <ModuleContainer
      title="Developer"
      trailing={
        <SegmentedControl<DevTab>
          label="Developer view"
          value={tab}
          options={[
            { value: "servers", label: "Servers" },
            { value: "time", label: "Time" },
            { value: "run", label: "Run" },
            { value: "widgets", label: "Widgets" },
          ]}
          onChange={(t) => useDevTools.getState().setTab(t)}
        />
      }
    >
      {tab === "servers" ? <Servers /> : tab === "time" ? <Time /> : tab === "run" ? <Run /> : <Widgets />}
    </ModuleContainer>
  );
}

function Servers() {
  const servers = useDevTools((s) => s.servers);
  const [status, setStatus] = useState<string | null>(null);
  // Look again while open (the background check is every 10 s).
  usePolled(() => ipc.devServers().then((list) => useDevTools.setState({ servers: list })), 3000);
  const run = (action: Promise<unknown>, done?: string) =>
    void action.then(
      () => setStatus(done ?? null),
      (e) => setStatus(message(e)),
    );

  if (servers.length === 0) {
    return (
      <EmptyState
        title="No dev servers running"
        body="Start one (npm run dev, vite, python manage.py runserver…) and it shows up here."
      />
    );
  }
  return (
    <div className="dev">
      {status && <p className="clips-status">{status}</p>}
      <ul className="clips-list" aria-label="Dev servers">
        {servers.map((s) => (
          <li key={s.port} className="clip">
            <button
              type="button"
              className="clip__main"
              title={`Open ${s.url}`}
              onClick={() => run(ipc.openExternal(s.url))}
            >
              <span className="dev-port">:{s.port}</span>
              <span className="clip__text">
                <span className="clip__title">{s.command ?? s.process}</span>
                <span className="clip__meta">
                  {s.process} · pid {s.pid}
                </span>
              </span>
            </button>
            <div className="clip__actions">
              <IconButton
                icon={ExternalLink}
                label={`Open ${s.url}`}
                size="sm"
                onClick={() => run(ipc.openExternal(s.url))}
              />
              <IconButton
                icon={Copy}
                label="Copy URL"
                size="sm"
                onClick={() => run(navigator.clipboard.writeText(s.url), `Copied ${s.url}`)}
              />
              <ConfirmButton
                confirmLabel="Stop it?"
                variant="danger"
                onConfirm={() => run(ipc.devStopServer(s.pid), `Stopped :${s.port}`)}
              >
                <Square size={12} aria-hidden />
              </ConfirmButton>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

function Time() {
  const stats = usePolled(() => ipc.codingStats(), 60_000);
  if (!stats) return <div className="dev" />;
  const total = stats.today.reduce((sum, p) => sum + p.minutes, 0);
  const peak = Math.max(30, ...stats.week);
  const today = new Date().getDay();
  return (
    <div className="dev dev-time">
      <div className="dev-time__head">
        <div>
          <p className="dev-time__big">{hoursMinutes(total)}</p>
          <p className="dev-time__sub">coded today</p>
        </div>
        <div className="dev-time__streak" title="Days in a row with 15+ minutes of coding">
          <Flame size={16} aria-hidden />
          {stats.streak} {stats.streak === 1 ? "day" : "days"}
        </div>
      </div>
      <div className="dev-time__body">
        <ul className="dev-time__projects" aria-label="Projects today">
          {stats.today.length === 0 ? (
            <li className="dev-time__empty">
              Open a project in VS Code, Cursor or a JetBrains IDE and it's counted.
            </li>
          ) : (
            stats.today.slice(0, 5).map((p) => (
              <li key={p.project}>
                <span className="dev-time__project">{p.project}</span>
                <span className="dev-time__minutes">{hoursMinutes(p.minutes)}</span>
              </li>
            ))
          )}
        </ul>
        <div className="dev-time__week" aria-label="Last 7 days">
          {stats.week.map((m, i) => (
            <div key={i} className="dev-time__day" title={hoursMinutes(m)}>
              <span className="dev-time__bar" style={{ height: `${Math.max(4, (m / peak) * 100)}%` }} />
              <span className="dev-time__label">{DAYS[(today - 6 + i + 7) % 7]}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function Run() {
  const [language, setLanguage] = useState<Language>("javascript");
  const [code, setCode] = useState("");
  const [result, setResult] = useState<RunResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);

  const go = async () => {
    if (!code.trim() || running) return;
    setRunning(true);
    setError(null);
    try {
      setResult(await ipc.devRun(language, code));
    } catch (e) {
      setError(message(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="dev dev-run">
      <div className="clips-bar">
        <SegmentedControl<Language>
          label="Language"
          value={language}
          options={[
            { value: "javascript", label: "JavaScript" },
            { value: "python", label: "Python" },
          ]}
          onChange={setLanguage}
        />
        <span className="clips-bar__spacer" />
        <span className="dev-run__hint">Ctrl+Enter</span>
        <Button variant="primary" onClick={() => void go()} disabled={running || !code.trim()}>
          <Play size={12} aria-hidden /> {running ? "Running…" : "Run"}
        </Button>
      </div>
      <textarea
        className="dev-run__code"
        aria-label="Code"
        spellCheck={false}
        value={code}
        placeholder={language === "javascript" ? "[1, 2, 3].map(x => x * 2)" : "sum(range(10))"}
        onChange={(e) => setCode(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            void go();
          }
          // Tab indents instead of leaving the box.
          if (e.key === "Tab") {
            e.preventDefault();
            const el = e.currentTarget;
            const at = el.selectionStart;
            setCode(code.slice(0, at) + "  " + code.slice(el.selectionEnd));
            requestAnimationFrame(() => el.setSelectionRange(at + 2, at + 2));
          }
        }}
      />
      {(result || error) && (
        <pre className={`dev-run__out${error || (result && !result.ok) ? " dev-run__out--error" : ""}`}>
          {error ?? (result!.output || "(no output)")}
          {result && !error && <span className="dev-run__time"> · {result.millis} ms</span>}
        </pre>
      )}
    </div>
  );
}

function Widgets() {
  const widgets = usePolled(() => ipc.devWidgets(), 60_000);
  if (!widgets) return <div className="dev" />;
  if (widgets.length === 0) {
    return (
      <EmptyState
        title="No widgets yet"
        body="Add one in Settings → Developer: a URL that returns JSON, or a PowerShell command. Its output shows here."
        action={<Button onClick={() => void ipc.openSettingsWindow("developer")}>Add a widget</Button>}
      />
    );
  }
  return (
    <div className="dev dev-widgets">
      {widgets.map((w) => (
        <div key={w.id} className="dev-widget">
          <p className="dev-widget__name">{w.title ?? w.name}</p>
          {w.error ? (
            <p className="dev-widget__error">{w.error}</p>
          ) : (
            <>
              {w.value && <p className="dev-widget__value">{w.value}</p>}
              {w.subtitle && <p className="dev-widget__sub">{w.subtitle}</p>}
              {w.items.length > 0 && (
                <dl className="dev-widget__items">
                  {w.items.map((item) => (
                    <div key={item.label}>
                      <dt>{item.label}</dt>
                      <dd>{item.value}</dd>
                    </div>
                  ))}
                </dl>
              )}
              {w.text && <pre className="dev-widget__text">{w.text}</pre>}
            </>
          )}
        </div>
      ))}
    </div>
  );
}
