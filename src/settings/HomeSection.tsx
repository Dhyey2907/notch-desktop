import { SegmentedControl, Surface, Switch } from "../design/components";
import { minutesToTime, timeToMinutes } from "../shell/behavior";
import { useSettings } from ".";
import { Row, TimeField } from "./controls";
import { patch, resetGroups } from "./patch";
import { PowerScheduleSetting } from "./PowerScheduleSetting";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

const TILES: { id: string; label: string; description: string }[] = [
  { id: "desktops", label: "Desktop switcher", description: "Switch Windows desktops from the top of Home." },
  { id: "system", label: "System", description: "CPU, memory and battery rings." },
  { id: "controls", label: "Sound & display", description: "Volume and brightness bars you can drag." },
  { id: "screen", label: "Screen time", description: "How long you've used the PC today." },
  { id: "weather", label: "Weather", description: "Uses Windows location." },
];

export function HomeSection() {
  const home = useSettings((s) => s.settings?.home);
  const day = useSettings((s) => s.settings?.day);
  const desk = useSettings((s) => s.settings?.desk);
  if (!home || !day || !desk) return null;
  const toggleDay = (d: number) =>
    patch("day", {
      workDays: day.workDays.includes(d) ? day.workDays.filter((x) => x !== d) : [...day.workDays, d].sort(),
    });
  const toggle = (id: string, on: boolean) =>
    patch("home", {
      hiddenTiles: on ? home.hiddenTiles.filter((t) => t !== id) : [...home.hiddenTiles, id],
    });

  return (
    <Section title="Home" lead={pageLead("home")} onReset={() => void resetGroups(["home", "day"])}>
      <Surface>
        <p id="setting-tiles" className="settings__row-label settings__surface-title">
          Tiles
        </p>
        {TILES.map((t) => (
          <Switch
            key={t.id}
            label={t.label}
            description={t.description}
            checked={!home.hiddenTiles.includes(t.id)}
            onChange={(on) => toggle(t.id, on)}
          />
        ))}
      </Surface>
      <Surface>
        <Row label="Temperature">
          <SegmentedControl
            label="Temperature"
            value={home.fahrenheit ? "f" : "c"}
            options={[
              { value: "c", label: "°C" },
              { value: "f", label: "°F" },
            ]}
            onChange={(v) => patch("home", { fahrenheit: v === "f" })}
          />
        </Row>
        <Row label="Clock" description="Used in the bubble, the game strip and elsewhere in the notch.">
          <SegmentedControl
            label="Clock"
            value={home.clock24h ? "24" : "12"}
            options={[
              { value: "12", label: "6:30 pm" },
              { value: "24", label: "18:30" },
            ]}
            onChange={(v) => patch("home", { clock24h: v === "24" })}
          />
        </Row>
      </Surface>
      <h2 id="setting-workday" className="settings__subhead">
        Workday
      </h2>
      <Surface>
        <Row
          label="Work hours"
          description="Home shows how much of the workday is left and when the weekend starts."
        >
          <span className="settings-inline">
            <TimeField
              label="Work starts"
              value={minutesToTime(day.startMin)}
              onChange={(t) => {
                const startMin = timeToMinutes(t);
                if (startMin !== null && startMin < day.endMin) patch("day", { startMin });
              }}
            />
            <span className="settings__row-description">to</span>
            <TimeField
              label="Work ends"
              value={minutesToTime(day.endMin)}
              onChange={(t) => {
                const endMin = timeToMinutes(t);
                if (endMin !== null && endMin > day.startMin) patch("day", { endMin });
              }}
            />
          </span>
        </Row>
        <Row label="Work days">
          <span className="settings-days" role="group" aria-label="Work days">
            {["M", "T", "W", "T", "F", "S", "S"].map((letter, i) => (
              <button
                key={i}
                type="button"
                aria-pressed={day.workDays.includes(i + 1)}
                aria-label={["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"][i]}
                className={`settings-days__day${day.workDays.includes(i + 1) ? " settings-days__day--on" : ""}`}
                onClick={() => toggleDay(i + 1)}
              >
                {letter}
              </button>
            ))}
          </span>
        </Row>
        <Switch
          label="Workday line under the bubble"
          description="A thin line under the notch fills up as the workday goes by."
          checked={!day.noWorkdayBar}
          onChange={(on) => patch("day", { noWorkdayBar: !on })}
        />
        <div id="setting-share-google-with-the-planner">
          <Switch
            label="Share Google data with the planner"
            description="Lets the AI day planner see today's Google Calendar meetings and Classroom deadlines (titles and times). They're sent to the AI you chose in Settings → AI (e.g. Claude or Codex) only when a plan is made — never stored or used for anything else."
            checked={day.shareGoogle}
            onChange={(on) => patch("day", { shareGoogle: on })}
          />
        </div>
        <div id="setting-plan-my-day-each-morning">
          <Switch
            label="Plan my day each morning"
            description="On work days, once you're at the PC after work starts, Claude Code suggests a schedule from your tasks, meetings and deadlines (Day module). Only titles and times are sent."
            checked={!day.noAutoPlan}
            onChange={(on) => patch("day", { noAutoPlan: !on })}
          />
        </div>
        <div id="setting-routines-from-habits">
          <Switch
            label="Suggest routines from my habits"
            description="When the same apps keep starting together at about the same time, offer a Desk routine that opens them for you. Only which apps came to the front and when is noted, kept 30 days on this PC."
            checked={!desk.noHabitRoutines}
            onChange={(on) => patch("desk", { noHabitRoutines: !on })}
          />
        </div>
        <div id="setting-morning-briefing">
          <Switch
            label="Morning briefing"
            description="The first time you're at the PC after 5 am: the weather, your meetings, open tasks and unread mail in one card."
            checked={!day.noBriefing}
            onChange={(on) => patch("day", { noBriefing: !on })}
          />
        </div>
        <div id="setting-evening-review">
          <Switch
            label="Evening review"
            description="Once a day, a card sums up your day: tasks done, focus, coding time, screen time and meetings."
            checked={!day.noReview}
            onChange={(on) => patch("day", { noReview: !on })}
          />
          {!day.noReview && (
            <Row label="Show it at">
              <TimeField
                label="Evening review time"
                value={minutesToTime(day.reviewAt)}
                onChange={(t) => {
                  const m = timeToMinutes(t);
                  if (m !== null) patch("day", { reviewAt: m });
                }}
              />
            </Row>
          )}
        </div>
      </Surface>
      <PowerScheduleSetting />
    </Section>
  );
}
