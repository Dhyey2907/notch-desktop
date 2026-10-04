// Usage dashboard (architecture.md §7 "Usage dashboard visual direction"): Overview / Models
// tabs, All / 30d / 7d range, six metric cards, a blue-step daily heatmap, and a muted caption.
// Every number is a local estimate from this PC's transcripts, and is labelled as such.
import { useEffect, useState } from "react";

import type { LimitWindow } from "../../core/generated/LimitWindow";

import type { UsageRange } from "../../core/generated/UsageRange";
import type { UsageSummary } from "../../core/generated/UsageSummary";
import { ipc } from "../../core/ipc";
import {
  Button,
  EmptyState,
  Heatmap,
  ModuleContainer,
  Pill,
  Progress,
  SegmentedControl,
  type HeatmapCell,
} from "../../design/components";
import { heatmapLevel } from "../../design/heatmap";
import {
  agentName,
  comparisonText,
  formatHour,
  formatTokens,
  limitLabel,
  prettyModel,
  primarySession,
  useClaude,
} from "./claudeStore";
import { limitTone, resetText, updatedText } from "./limits";
import "./claude.css";

type Tab = "overview" | "models";
const RANGE_DAYS: Record<UsageRange, number | null> = { all: null, days30: 30, days7: 7 };

function useUsage(range: UsageRange) {
  const version = useClaude((s) => s.usageVersion);
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  useEffect(() => {
    let cancelled = false;
    ipc.claudeUsage(range).then(
      (s) => !cancelled && setSummary(s),
      () => !cancelled && setSummary(null),
    );
    return () => {
      cancelled = true;
    };
  }, [range, version]);
  return summary;
}

export default function ClaudeExpanded() {
  const status = useClaude((s) => s.status);
  const [tab, setTab] = useState<Tab>("overview");
  const [range, setRange] = useState<UsageRange>("days30");
  const summary = useUsage(range);

  if (status && !status.index.transcriptsFound) {
    return (
      <ModuleContainer title="Claude Code">
        <EmptyState
          title="Usage unavailable"
          body="No Claude Code transcripts were found on this PC. Use Claude Code here and your activity will appear."
        />
      </ModuleContainer>
    );
  }

  const trailing = (
    <>
      <LiveBadge />
      <SegmentedControl<Tab>
        label="Claude view"
        value={tab}
        options={[
          { value: "overview", label: "Overview" },
          { value: "models", label: "Models" },
        ]}
        onChange={setTab}
      />
      <SegmentedControl<UsageRange>
        label="Range"
        value={range}
        options={[
          { value: "all", label: "All" },
          { value: "days30", label: "30d" },
          { value: "days7", label: "7d" },
        ]}
        onChange={setRange}
      />
    </>
  );

  return (
    <ModuleContainer title="Claude Code" trailing={trailing}>
      {!status?.index.ready && !summary ? (
        <EmptyState title="Reading your Claude Code history…" body="This takes a moment the first time." />
      ) : summary ? (
        tab === "overview" ? (
          <Overview summary={summary} />
        ) : (
          <Models summary={summary} />
        )
      ) : (
        <EmptyState title="Usage unavailable" body="Couldn't read the usage index. Try again in a moment." />
      )}
    </ModuleContainer>
  );
}

function LiveBadge() {
  const status = useClaude((s) => s.status);
  const session = status ? primarySession(status.live) : null;
  if (status?.quota) {
    const at = new Date(status.quota.resetsAt).toLocaleTimeString(undefined, {
      hour: "numeric",
      minute: "2-digit",
    });
    return <Pill tone="warning">{`${limitLabel(status.quota.limitType)} limit · resets ${at}`}</Pill>;
  }
  if (!session || session.state === "idle") return null;
  const who = agentName(session.agent);
  return session.state === "waiting" ? (
    <Pill tone="warning">{`${who} is waiting for you`}</Pill>
  ) : (
    <Pill tone="info">{`${who} working${session.project ? ` · ${session.project}` : ""}`}</Pill>
  );
}

function Card({ label, value, title }: { label: string; value: string; title?: string }) {
  return (
    <div className="usage-card" title={title}>
      <span className="usage-card__label">{label}</span>
      <span className="usage-card__value">{value}</span>
    </div>
  );
}

function Overview({ summary }: { summary: UsageSummary }) {
  const days = RANGE_DAYS[summary.range];
  const max = Math.max(0, ...summary.daily.map((d) => d.tokens));
  const cells: HeatmapCell[] = summary.daily.map((d) => ({
    date: d.date,
    value: d.tokens,
    level: heatmapLevel(d.tokens, max),
  }));
  const caption =
    (days && comparisonText(summary.totalTokens, summary.previousPeriodTokens, days)) ||
    (summary.topProject ? `Most active in ${summary.topProject}` : null);

  return (
    <div className="usage">
      <div className="usage__cards">
        <Card label="Sessions" value={summary.sessions.toLocaleString()} />
        <Card label="Messages" value={summary.messages.toLocaleString()} title="Prompts you sent" />
        <Card
          label="Total tokens"
          value={formatTokens(summary.totalTokens)}
          title={`Input ${formatTokens(summary.inputTokens)} · output ${formatTokens(summary.outputTokens)} · cache read ${formatTokens(summary.cacheReadTokens)} · cache write ${formatTokens(summary.cacheCreationTokens)}`}
        />
        <Card label="Active days" value={summary.activeDays.toLocaleString()} />
        <Card label="Peak hour" value={summary.peakHour != null ? formatHour(summary.peakHour) : "—"} />
        <Card
          label="Favorite model"
          value={summary.favoriteModel ? prettyModel(summary.favoriteModel) : "—"}
          title={summary.favoriteModel ?? undefined}
        />
      </div>
      <div className="usage__row">
        <Heatmap cells={cells} label="Claude Code tokens per day, last 26 weeks" unit="tokens" />
        <PlanLimits />
      </div>
      <p className="usage__caption">
        {caption && <span>{caption} · </span>}
        <span className="usage__source">Local estimate from Claude Code transcripts on this PC</span>
      </p>
    </div>
  );
}

function Models({ summary }: { summary: UsageSummary }) {
  if (summary.models.length === 0) {
    return <EmptyState title="No usage in this range" body="Pick a longer range to see your models." />;
  }
  const max = Math.max(...summary.models.map((m) => m.tokens));
  return (
    <div className="usage">
      <ul className="usage-models">
        {summary.models.slice(0, 6).map((m) => (
          <li key={m.model} className="usage-models__row">
            <span className="usage-models__name" title={m.model}>
              {prettyModel(m.model)}
            </span>
            <Progress
              label={`${prettyModel(m.model)} share of tokens`}
              value={m.tokens}
              max={max}
              tone="info"
            />
            <span className="usage-models__value">{formatTokens(m.tokens)}</span>
            <span className="usage-models__meta">
              {m.messages.toLocaleString()} {m.messages === 1 ? "msg" : "msgs"}
            </span>
          </li>
        ))}
      </ul>
      <p className="usage__caption usage__source">Local estimate from Claude Code transcripts on this PC</p>
    </div>
  );
}

/** Re-render on a slow tick so countdowns stay honest without a busy timer. */
function useMinuteClock() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 30_000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

function LimitRow({ label, window: w, now }: { label: string; window: LimitWindow; now: number }) {
  const percent = Math.round(w.usedPercent);
  return (
    <div className="limits__row">
      <div className="limits__line">
        <span className="limits__label">{label}</span>
        <span className="limits__value">{percent}%</span>
      </div>
      <Progress label={`${label} used`} value={w.usedPercent} max={100} tone={limitTone(w.usedPercent)} />
      <span className="limits__reset">{resetText(w.resetsAt, now)}</span>
    </div>
  );
}

/** 5-hour and weekly plan limits, as last reported by Claude Code's status line. */
function PlanLimits() {
  const status = useClaude((s) => s.status);
  const now = useMinuteClock();
  const limits = status?.limits;

  let body;
  if (limits) {
    body = (
      <>
        {limits.fiveHour && <LimitRow label="5-hour limit" window={limits.fiveHour} now={now} />}
        {limits.sevenDay && <LimitRow label="Weekly · all models" window={limits.sevenDay} now={now} />}
        <span className="limits__updated">{updatedText(limits.updatedAt, now)}</span>
      </>
    );
  } else if (status?.hooks.statusLine) {
    body = (
      <p className="limits__hint">
        Limits arrive when you use Claude Code in a terminal — the desktop app doesn't share them.
      </p>
    );
  } else {
    body = (
      <>
        <p className="limits__hint">See your 5-hour and weekly limits here.</p>
        <Button onClick={() => void ipc.openSettingsWindow("ai")}>Set up</Button>
      </>
    );
  }
  return (
    <section className="limits" aria-label="Plan usage limits">
      <h3 className="limits__title">Plan usage limits</h3>
      {body}
    </section>
  );
}
