// Settings → Your day → Power schedule: sleep at a time (only once you've been away a while)
// and wake at a time, on the days you pick.
import type { PowerSchedule } from "../core/generated/PowerSchedule";
import { Surface, Switch } from "../design/components";
import { minutesToTime, timeToMinutes } from "../shell/behavior";
import { useSettings } from ".";
import { Row, Select, TimeField } from "./controls";

const DAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

export function PowerScheduleSetting() {
  const power = useSettings((s) => s.settings?.power);
  if (!power) return null;
  const set = (change: Partial<PowerSchedule>) =>
    void useSettings.getState().update((s) => ({ ...s, power: { ...s.power, ...change } }));
  const toggleDay = (d: number) =>
    set({ days: power.days.includes(d) ? power.days.filter((x) => x !== d) : [...power.days, d].sort() });

  return (
    <Surface>
      <p id="setting-power-schedule" className="settings__row-label settings__surface-title">
        Power schedule
      </p>
      <Switch
        label="Sleep at night"
        description="Puts the PC to sleep at this time if nothing was touched for a while (it waits if you're still using it, up to an hour)."
        checked={power.sleepAt !== undefined}
        onChange={(on) => set({ sleepAt: on ? 23 * 60 : undefined })}
      />
      {power.sleepAt !== undefined && (
        <>
          <Row label="Sleep at">
            <TimeField
              label="Sleep at"
              value={minutesToTime(power.sleepAt)}
              onChange={(t) => {
                const m = timeToMinutes(t);
                if (m !== null) set({ sleepAt: m });
              }}
            />
          </Row>
          <Row label="Once you've been away for">
            <Select
              label="Once you've been away for"
              value={String(power.sleepIfIdleMin)}
              options={[5, 10, 15, 30, 60].map((m) => ({ value: String(m), label: `${m} min` }))}
              onChange={(v) => set({ sleepIfIdleMin: Number(v) })}
            />
          </Row>
        </>
      )}
      <Switch
        label="Wake in the morning"
        description="Wakes the PC from sleep at this time (a Windows scheduled task). Needs “Allow wake timers” on in your power plan; it can't power on a PC that's shut down."
        checked={power.wakeAt !== undefined}
        onChange={(on) => set({ wakeAt: on ? 7 * 60 + 30 : undefined })}
      />
      {power.wakeAt !== undefined && (
        <Row label="Wake at">
          <TimeField
            label="Wake at"
            value={minutesToTime(power.wakeAt)}
            onChange={(t) => {
              const m = timeToMinutes(t);
              if (m !== null) set({ wakeAt: m });
            }}
          />
        </Row>
      )}
      {(power.sleepAt !== undefined || power.wakeAt !== undefined) && (
        <Row label="On">
          <span className="settings-days" role="group" aria-label="Power schedule days">
            {["M", "T", "W", "T", "F", "S", "S"].map((letter, i) => (
              <button
                key={i}
                type="button"
                aria-pressed={power.days.includes(i + 1)}
                aria-label={DAY_NAMES[i]}
                className={`settings-days__day${power.days.includes(i + 1) ? " settings-days__day--on" : ""}`}
                onClick={() => toggleDay(i + 1)}
              >
                {letter}
              </button>
            ))}
          </span>
        </Row>
      )}
    </Surface>
  );
}
