import { X } from "lucide-react";
import { useState } from "react";

import type { WidgetConfig } from "../core/generated/WidgetConfig";
import { Button, IconButton, SegmentedControl, Surface, Switch, TextField } from "../design/components";
import { useSettings } from ".";
import { Row, Select } from "./controls";
import { patch } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

const REFRESH = [
  { value: "30", label: "30 seconds" },
  { value: "60", label: "1 minute" },
  { value: "300", label: "5 minutes" },
  { value: "900", label: "15 minutes" },
  { value: "3600", label: "1 hour" },
];

export function DeveloperSection() {
  const dev = useSettings((s) => s.settings?.dev);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<"url" | "command">("url");
  const [source, setSource] = useState("");
  const [refresh, setRefresh] = useState("60");
  const [error, setError] = useState<string | null>(null);
  if (!dev) return null;

  const add = () => {
    if (kind === "url" && !/^https?:\/\//.test(source.trim())) {
      setError("The URL should start with https://");
      return;
    }
    const widget: WidgetConfig = {
      id: `w${Date.now().toString(36)}`,
      name: name.trim(),
      kind,
      source: source.trim(),
      refreshSecs: Number(refresh),
    };
    void useSettings
      .getState()
      .update((s) => ({ ...s, dev: { ...s.dev, widgets: [...s.dev.widgets, widget] } }))
      .then(
        () => {
          setName("");
          setSource("");
          setError(null);
        },
        () => setError("Couldn't add that widget — check the name and source."),
      );
  };

  return (
    <Section title="Developer" lead={pageLead("developer")}>
      <Surface>
        <div id="setting-tell-me-when-a-dev-server-stops">
          <Switch
            label="Tell me when a dev server stops"
            description="Servers like vite, next dev or a Python server on localhost show in Developer → Servers. If one that was running stops (or crashes), the notch says so."
            checked={!dev.noServerAlerts}
            onChange={(on) => patch("dev", { noServerAlerts: !on })}
          />
        </div>
        <div id="setting-count-coding-time">
          <Switch
            label="Count coding time"
            description="While VS Code, Cursor, a JetBrains IDE or another editor is in front and you're active, minutes are counted per project (the name comes from the window title). Kept on this PC."
            checked={!dev.noCodingTracker}
            onChange={(on) => patch("dev", { noCodingTracker: !on })}
          />
        </div>
        <div id="setting-claude-spend-cap">
          <Row
            label="Claude daily spend cap"
            description="Today's Claude Code use, priced at API rates (your plan may not bill it): a warning at 80% and when it passes the cap."
          >
            <Select
              label="Claude daily spend cap"
              value={String(dev.claudeDailyCapUsd)}
              options={[
                { value: "0", label: "Off" },
                ...[5, 10, 20, 50, 100, 200].map((usd) => ({ value: String(usd), label: `${usd}` })),
              ]}
              onChange={(v) => patch("dev", { claudeDailyCapUsd: Number(v) })}
            />
          </Row>
        </div>
      </Surface>
      <Surface>
        <p id="setting-widgets" className="settings__row-label settings__surface-title">
          Widgets
        </p>
        <p className="settings__note">
          Show your own numbers in Developer → Widgets. A URL can return JSON like{" "}
          <code>
            {'{"value": "42 ms", "subtitle": "API latency", "items": [{"label": "CPU", "value": "12%"}]}'}
          </code>{" "}
          or plain text; a command runs in PowerShell and shows what it prints.
        </p>
        {dev.widgets.length > 0 && (
          <ul className="snippets" aria-label="Widgets">
            {dev.widgets.map((w) => (
              <li key={w.id} className="snippets__item">
                <kbd className="settings__kbd">{w.name}</kbd>
                <span className="snippets__body" title={w.source}>
                  {w.kind === "url" ? "URL" : "Command"} · {w.source}
                </span>
                <IconButton
                  icon={X}
                  label={`Remove ${w.name}`}
                  size="sm"
                  onClick={() => patch("dev", { widgets: dev.widgets.filter((x) => x.id !== w.id) })}
                />
              </li>
            ))}
          </ul>
        )}
        <form
          className="settings__row settings__row--stacked"
          onSubmit={(e) => {
            e.preventDefault();
            add();
          }}
        >
          <TextField label="Name" value={name} onChange={setName} placeholder="API status" />
          <Row label="Gets its data from">
            <SegmentedControl<"url" | "command">
              label="Gets its data from"
              value={kind}
              options={[
                { value: "url", label: "A URL" },
                { value: "command", label: "A command" },
              ]}
              onChange={setKind}
            />
          </Row>
          <TextField
            label={kind === "url" ? "URL" : "PowerShell command"}
            value={source}
            onChange={setSource}
            placeholder={kind === "url" ? "https://api.example.com/health" : "(Get-ChildItem C:\\dev).Count"}
            description={
              kind === "command" ? "Runs on this PC with your permissions, up to 10 seconds." : undefined
            }
          />
          <Row label="Refresh every">
            <Select label="Refresh every" value={refresh} options={REFRESH} onChange={setRefresh} />
          </Row>
          <div className="settings__buttons settings__buttons--start">
            <Button variant="primary" type="submit" disabled={!name.trim() || !source.trim()}>
              Add widget
            </Button>
          </div>
          {error && <p className="settings__note settings__note--warning">{error}</p>}
        </form>
      </Surface>
    </Section>
  );
}
