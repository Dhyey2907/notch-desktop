import { Surface, Switch } from "../design/components";
import { currentStreak, STREAK_BREAKS } from "../focus/breaks";
import { useSettings } from ".";
import { Row, Slider } from "./controls";
import { patch } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";
import { useRgbStatus } from "./useRgbStatus";

const minutes = (v: number) =>
  v >= 60 && v % 60 === 0 ? `${v / 60} h` : v >= 60 ? `${Math.floor(v / 60)} h ${v % 60}` : `${v} min`;

export function FocusSection() {
  const focus = useSettings((s) => s.settings?.focus);
  const breaks = useSettings((s) => s.settings?.breaks);
  const fun = useSettings((s) => s.settings?.fun);
  const rgb = useRgbStatus(fun?.rgb ?? false);
  if (!focus || !breaks || !fun) return null;
  const master = focus.master || 0.6;
  // The settings page is opened fresh, so reading the date while rendering is fine here.
  const streak = currentStreak(breaks, new Date());

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

      <h2 id="setting-healthy-breaks" className="settings__subhead">
        Healthy breaks
      </h2>
      <p className="settings__lead">
        Reminders count only time you're actually at the PC; stepping away for 5 minutes counts as a break.
        They wait while you focus, play, sit in a meeting or have quiet hours.{" "}
        {streak > 0
          ? `The pet's streak: ${streak} ${streak === 1 ? "day" : "days"} in a row (best ${breaks.best}).`
          : `Take ${STREAK_BREAKS} breaks in a day to start a streak with the pet.`}
      </p>
      <Surface>
        <Switch
          label="Rest your eyes"
          description="Every so often, look 20 feet away for 20 seconds (the 20-20-20 rule)."
          checked={!breaks.noEyes}
          onChange={(on) => patch("breaks", { noEyes: !on })}
        />
        {!breaks.noEyes && (
          <Row label="Eye break every">
            <Slider
              label="Eye break every"
              value={breaks.eyeMin}
              min={10}
              max={60}
              step={5}
              onChange={(v) => patch("breaks", { eyeMin: v })}
              format={minutes}
            />
          </Row>
        )}
        <Switch
          label="Stretch"
          description="Stand up and move for a minute."
          checked={!breaks.noStretch}
          onChange={(on) => patch("breaks", { noStretch: !on })}
        />
        {!breaks.noStretch && (
          <Row label="Stretch every">
            <Slider
              label="Stretch every"
              value={breaks.stretchMin}
              min={30}
              max={180}
              step={15}
              onChange={(v) => patch("breaks", { stretchMin: v })}
              format={minutes}
            />
          </Row>
        )}
        <Switch
          label="Drink water"
          description="A reminder to have a glass of water."
          checked={!breaks.noWater}
          onChange={(on) => patch("breaks", { noWater: !on })}
        />
        {!breaks.noWater && (
          <Row label="Water every">
            <Slider
              label="Water every"
              value={breaks.waterMin}
              min={30}
              max={240}
              step={15}
              onChange={(v) => patch("breaks", { waterMin: v })}
              format={minutes}
            />
          </Row>
        )}
      </Surface>

      <h2 id="setting-fun" className="settings__subhead">
        Fun
      </h2>
      <Surface>
        <div id="setting-pet-walk">
          <Switch
            label="Pet walks the taskbar"
            description="After two minutes away, the pet strolls along the top of the taskbar. It goes the moment you're back."
            checked={fun.petWalk}
            onChange={(on) => patch("fun", { petWalk: on })}
          />
        </div>
        <div id="setting-rgb-lights">
          <Switch
            label="RGB lights"
            description="Your PC's lights take the notch's color, and pulse with the beat while music plays. Needs the free OpenRGB app with its SDK server turned on."
            checked={fun.rgb}
            onChange={(on) => patch("fun", { rgb: on })}
          />
        </div>
        {fun.rgb && (
          <Row
            label="OpenRGB"
            description={
              rgb?.state === "connected"
                ? `Connected · ${rgb.devices} ${rgb.devices === 1 ? "device" : "devices"}`
                : rgb?.state === "notRunning"
                  ? "Not found. Open OpenRGB and turn on its SDK server (SDK Server tab → Start Server)."
                  : "Looking for OpenRGB…"
            }
          >
            <span className="settings-rgb-dot" data-ok={rgb?.state === "connected"} aria-hidden />
          </Row>
        )}
      </Surface>
    </Section>
  );
}
