import { Surface, Switch } from "../design/components";
import { useSettings } from ".";
import { Row, Slider } from "./controls";
import { patch } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

export function FocusSection() {
  const focus = useSettings((s) => s.settings?.focus);
  if (!focus) return null;
  const master = focus.master || 0.6;

  return (
    <Section title="Focus" lead={pageLead("focus")}>
      <p className="settings__lead">
        Start a focus session from To-do → Focus. While it runs, ordinary notifications wait and show as one
        summary at the end. For a daily quiet time, use Quiet hours in Behavior.
      </p>
      <Surface>
        <div id="setting-pet">
          <Switch
            label="Pet"
            description="A small character in the bubble that reacts to focus, music and the time of day."
            checked={!focus.hidePet}
            onChange={(on) => patch("focus", { hidePet: !on })}
          />
        </div>
        <Row label="Ambient volume" description="Overall volume of rain, ocean and the other focus sounds.">
          <Slider
            label="Ambient volume"
            value={Math.round(master * 100)}
            min={5}
            max={100}
            step={5}
            onChange={(v) => patch("focus", { master: v / 100 })}
            format={(v) => `${v}%`}
          />
        </Row>
      </Surface>
    </Section>
  );
}
