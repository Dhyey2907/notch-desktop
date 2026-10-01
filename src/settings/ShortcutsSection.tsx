import { useEffect, useState } from "react";

import type { ShortcutProblem } from "../core/generated/ShortcutProblem";
import type { ShortcutSettings } from "../core/generated/ShortcutSettings";
import type { Speed } from "../core/generated/Speed";
import { ipc } from "../core/ipc";
import { SegmentedControl, Surface, Switch } from "../design/components";
import { useSettings } from ".";
import { Row, ShortcutRecorder } from "./controls";
import { patch, resetGroups } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";
import { prettyShortcut } from "./shortcutText";

const SHORTCUTS: { key: keyof ShortcutSettings; label: string; description: string; fallback: string }[] = [
  { key: "open", label: "Open or close the notch", description: "From any app.", fallback: "Ctrl+Alt+N" },
  {
    key: "ask",
    label: "Ask Claude",
    description: "Opens Ask with the app you were using ready to include.",
    fallback: "Ctrl+Alt+A",
  },
  {
    key: "clipboard",
    label: "Clipboard history",
    description: "Opens straight to what you copied.",
    fallback: "Ctrl+Alt+V",
  },
];

export function ShortcutsSection() {
  const keys = useSettings((s) => s.settings?.shortcuts);
  const gestures = useSettings((s) => s.settings?.gestures);
  const [problems, setProblems] = useState<ShortcutProblem[]>([]);

  // The notch registers shortcuts as soon as they change; ask what it couldn't use.
  useEffect(() => {
    if (!keys) return;
    const timer = window.setTimeout(() => ipc.shortcutProblems().then(setProblems, () => {}), 400);
    return () => window.clearTimeout(timer);
  }, [keys]);

  if (!keys || !gestures) return null;

  return (
    <Section
      title="Shortcuts & gestures"
      lead={pageLead("shortcuts")}
      onReset={() => void resetGroups(["shortcuts", "gestures"])}
    >
      <Surface>
        {SHORTCUTS.map((s) => {
          const problem = problems.find((p) => p.name === s.key);
          return (
            <div key={s.key}>
              <Row label={s.label} description={s.description}>
                <ShortcutRecorder
                  label={s.label}
                  value={keys[s.key]}
                  fallback={s.fallback}
                  onChange={(value) => patch("shortcuts", { [s.key]: value })}
                />
              </Row>
              {problem && (
                <p className="settings__note settings__note--warning" role="alert">
                  {problem.reason === "taken"
                    ? `${prettyShortcut(problem.keys)} is already used by another app or shortcut. Pick a different one.`
                    : `${problem.keys} isn't a key combination Windows can use. Pick a different one.`}
                </p>
              )}
            </div>
          );
        })}
        <p className="settings__note">
          Click a shortcut, then press the new keys. Backspace turns it off; Esc cancels.
        </p>
      </Surface>
      <Surface>
        <div id="setting-touchpad-swipes">
          <Switch
            label="Touchpad swipes"
            description="Two-finger swipes on the notch: up opens it, down closes it, left and right switch modules when open."
            checked={!gestures.disabled}
            onChange={(on) => patch("gestures", { disabled: !on })}
          />
        </div>
        <Row label="Swipe sensitivity" description="High reacts to a light flick; low needs a longer swipe.">
          <SegmentedControl<Speed>
            label="Swipe sensitivity"
            value={gestures.sensitivity}
            options={[
              { value: "slow", label: "Low" },
              { value: "normal", label: "Normal" },
              { value: "fast", label: "High" },
            ]}
            onChange={(sensitivity) => patch("gestures", { sensitivity })}
          />
        </Row>
        <div id="setting-swipe-to-change-songs">
          <Switch
            label="Swipe to change songs"
            description="Left and right on the closed notch play the previous or next song."
            checked={!gestures.noTrackSwipe}
            disabled={gestures.disabled}
            onChange={(on) => patch("gestures", { noTrackSwipe: !on })}
          />
        </div>
      </Surface>
    </Section>
  );
}
