import { Volume2 } from "lucide-react";

import type { AlertSound } from "../core/generated/AlertSound";
import type { BehaviorSettings } from "../core/generated/BehaviorSettings";
import type { FullscreenBehavior } from "../core/generated/FullscreenBehavior";
import type { Speed } from "../core/generated/Speed";
import { IconButton, SegmentedControl, Surface, Switch } from "../design/components";
import { playAlert } from "../shell/alertSound";
import { minutesToTime, timeToMinutes } from "../shell/behavior";
import { useSettings } from ".";
import { Row, Select, Slider, TimeField } from "./controls";
import { HudSetting } from "./HudSetting";
import { patch, resetGroups } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

const SPEEDS: { value: Speed; label: string }[] = [
  { value: "fast", label: "Fast" },
  { value: "normal", label: "Normal" },
  { value: "slow", label: "Slow" },
];

export function BehaviorSection() {
  const b = useSettings((s) => s.settings?.behavior);
  if (!b) return null;
  const set = (change: Partial<BehaviorSettings>) => patch("behavior", change);
  const setQuiet = (change: Partial<BehaviorSettings["quiet"]>) => set({ quiet: { ...b.quiet, ...change } });

  return (
    <Section
      title="Behavior"
      lead={pageLead("behavior")}
      onReset={() => void resetGroups(["behavior", "hud"])}
    >
      <Surface>
        <div id="setting-open-on-hover">
          <Switch
            label="Open on hover"
            description="Pointing at the notch opens it fully — no click needed. It closes when you move away."
            checked={b.openOnHover}
            onChange={(openOnHover) => set({ openOnHover })}
          />
        </div>
        <div id="setting-preview-on-hover">
          <Switch
            label="Preview on hover"
            description="Pointing at the bubble shows a small preview (the song, the next meeting)."
            checked={!b.noHoverPeek}
            disabled={b.openOnHover}
            onChange={(on) => set({ noHoverPeek: !on })}
          />
        </div>
        <Row
          label="Hover speed"
          description="How long you point before it reacts. Slow avoids accidental opens."
        >
          <SegmentedControl<Speed>
            label="Hover speed"
            value={b.hoverSpeed}
            options={SPEEDS}
            onChange={(hoverSpeed) => set({ hoverSpeed })}
          />
        </Row>
        <Row label="Close by itself" description="After you move the pointer away from the open notch.">
          <Select
            label="Close by itself"
            value={String(b.autoCloseSecs)}
            options={[
              { value: "0", label: "Only when I click away" },
              { value: "2", label: "After 2 seconds" },
              { value: "5", label: "After 5 seconds" },
              { value: "15", label: "After 15 seconds" },
              { value: "60", label: "After 1 minute" },
            ]}
            onChange={(v) => set({ autoCloseSecs: Number(v) })}
          />
        </Row>
      </Surface>
      <Surface>
        <Row label="Notifications stay for" description="How long a notification shows before it tucks away.">
          <SegmentedControl<Speed>
            label="Notifications stay for"
            value={b.cardTime}
            options={[
              { value: "fast", label: "Short" },
              { value: "normal", label: "Normal" },
              { value: "slow", label: "Long" },
            ]}
            onChange={(cardTime) => set({ cardTime })}
          />
        </Row>
        <Row label="Notification sound" description="A small sound when something new appears.">
          <span className="settings-inline">
            <SegmentedControl<AlertSound>
              label="Notification sound"
              value={b.sound}
              options={[
                { value: "off", label: "Off" },
                { value: "soft", label: "Soft" },
                { value: "chime", label: "Chime" },
              ]}
              onChange={(sound) => {
                set({ sound });
                void playAlert(sound, b.soundVolume);
              }}
            />
            <IconButton
              icon={Volume2}
              label="Play the sound"
              size="sm"
              disabled={b.sound === "off"}
              onClick={() => void playAlert(b.sound, b.soundVolume)}
            />
          </span>
        </Row>
        {b.sound !== "off" && (
          <Row label="Sound volume">
            <Slider
              label="Sound volume"
              value={Math.round(b.soundVolume * 100)}
              min={5}
              max={100}
              step={5}
              onChange={(v) => {
                set({ soundVolume: v / 100 });
                void playAlert(b.sound, v / 100);
              }}
              format={(v) => `${v}%`}
            />
          </Row>
        )}
      </Surface>
      <Surface>
        <div id="setting-quiet-hours">
          <Switch
            label="Quiet hours"
            description="Every day between these times, notifications wait and show as one summary afterwards. Approvals and timers still come through."
            checked={b.quiet.enabled}
            onChange={(enabled) => setQuiet({ enabled })}
          />
        </div>
        {b.quiet.enabled && (
          <div className="settings__row">
            <span className="settings__row-text">
              <span className="settings__row-label">From</span>
            </span>
            <span className="settings-inline">
              <TimeField
                label="Quiet from"
                value={minutesToTime(b.quiet.startMin)}
                onChange={(t) => {
                  const startMin = timeToMinutes(t);
                  if (startMin !== null) setQuiet({ startMin });
                }}
              />
              <span className="settings__row-description">to</span>
              <TimeField
                label="Quiet until"
                value={minutesToTime(b.quiet.endMin)}
                onChange={(t) => {
                  const endMin = timeToMinutes(t);
                  if (endMin !== null) setQuiet({ endMin });
                }}
              />
            </span>
          </div>
        )}
      </Surface>
      <Surface>
        <Row label="Over fullscreen apps" description="Games, videos and presentations." stacked>
          <SegmentedControl<FullscreenBehavior>
            label="Over fullscreen apps"
            value={b.fullscreen}
            options={[
              { value: "alerts", label: "Hide, show notifications" },
              { value: "hide", label: "Hide completely" },
              { value: "show", label: "Always show" },
            ]}
            onChange={(fullscreen) => set({ fullscreen })}
          />
        </Row>
        {b.fullscreen === "hide" && (
          <p className="settings__note">
            Approvals from Claude Code still appear, so an agent is never stuck waiting.
          </p>
        )}
      </Surface>
      <h2 className="settings__subhead">Pop-ups for system changes</h2>
      <HudSetting />
    </Section>
  );
}
