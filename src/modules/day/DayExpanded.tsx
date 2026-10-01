import { ExternalLink, RotateCw, Sparkles } from "lucide-react";
import { useState } from "react";

import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer, SegmentedControl } from "../../design/components";
import { useMinuteClock } from "../clipboard/useMinuteClock";
import { dueIn, planLine, useDay, type DayTab } from "./dayStore";
// The list rows share the Clipboard module's look (styles only).
import "../clipboard/clipboard.css";
import "./day.css";

export default function DayExpanded() {
  const tab = useDay((s) => s.tab);
  return (
    <ModuleContainer
      title="Day"
      trailing={
        <SegmentedControl<DayTab>
          label="Day view"
          value={tab}
          options={[
            { value: "plan", label: "Plan" },
            { value: "due", label: "Due" },
          ]}
          onChange={(t) => useDay.setState({ tab: t })}
        />
      }
    >
      {tab === "plan" ? <Plan /> : <Due />}
    </ModuleContainer>
  );
}

function Plan() {
  const state = useDay((s) => s.state);
  const [error, setError] = useState<string | null>(null);
  const plan = state?.plan;
  const planning = state?.planning ?? false;
  const make = () =>
    void ipc.dayPlan().then(
      () => setError(null),
      (e) => setError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "Couldn't make a plan."),
    );

  if (!plan) {
    return (
      <EmptyState
        title={planning ? "Planning your day…" : "Plan the rest of today"}
        body={
          error ??
          "Claude Code looks at your open tasks, today's meetings and what's due soon, and suggests a realistic schedule. Only titles and times are sent."
        }
        action={
          <Button variant="primary" onClick={make} disabled={planning}>
            <Sparkles size={13} aria-hidden /> {planning ? "Thinking…" : "Plan my day"}
          </Button>
        }
      />
    );
  }
  const made = new Date(plan.madeAt).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  return (
    <div className="day-plan">
      <ul className="day-plan__lines">
        {plan.text
          .split("\n")
          .filter((l) => l.trim())
          .map((line, i) => {
            const { time, text } = planLine(line);
            const focus = text.toLowerCase().startsWith("focus:");
            return (
              <li key={i} className={focus ? "day-plan__focus" : undefined}>
                {time && <span className="day-plan__time">{time}</span>}
                <span>{text}</span>
              </li>
            );
          })}
      </ul>
      <div className="day-plan__foot">
        <span>{error ?? `Made at ${made}`}</span>
        <IconButton
          icon={RotateCw}
          label={planning ? "Planning…" : "Plan again"}
          size="sm"
          disabled={planning}
          onClick={make}
        />
      </div>
    </div>
  );
}

function Due() {
  const state = useDay((s) => s.state);
  const now = useMinuteClock();
  if (!state || state.classroom === "off") {
    return (
      <EmptyState
        title="Assignment deadlines"
        body="Turn on Google Classroom in Settings → Calendar and sign in again to see work that's due, with a reminder the day before."
        action={
          <Button onClick={() => void ipc.openSettingsWindow("calendar")}>Open Calendar settings</Button>
        }
      />
    );
  }
  if (state.classroom === "needsSignIn") {
    return (
      <EmptyState
        title="Allow Classroom"
        body="Classroom is turned on, but Google hasn't been asked yet. Sign in with Google again in Settings → Calendar."
        action={<Button onClick={() => void ipc.openSettingsWindow("calendar")}>Sign in again</Button>}
      />
    );
  }
  if (state.assignments.length === 0) {
    return <EmptyState title="Nothing due" body="No Classroom work due in the next three weeks." />;
  }
  return (
    <ul className="clips-list" aria-label="Due soon">
      {state.assignments.map((a) => {
        const soon = new Date(a.due).getTime() - now < 86_400_000;
        return (
          <li key={a.id} className="clip">
            <div className="clip__main">
              <span className={`day-due${soon ? " day-due--soon" : ""}`}>{dueIn(a.due, now)}</span>
              <span className="clip__text">
                <span className="clip__title">{a.title}</span>
                <span className="clip__meta">{a.course}</span>
              </span>
            </div>
            {a.link && (
              <div className="clip__actions">
                <IconButton
                  icon={ExternalLink}
                  label="Open in Classroom"
                  size="sm"
                  onClick={() => void ipc.openExternal(a.link!)}
                />
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}
