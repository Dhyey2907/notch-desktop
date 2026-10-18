import type { HudSettings } from "../core/generated/HudSettings";
import { Surface, Switch } from "../design/components";
import { useSettings } from ".";
import { Row, Select } from "./controls";

/** Settings → Behavior: which system changes pop up in the notch. */
export function HudSetting() {
  const hud = useSettings((s) => s.settings?.hud);
  if (!hud) return null;
  const set = (change: Partial<HudSettings>) =>
    void useSettings.getState().update((s) => ({ ...s, hud: { ...s.hud, ...change } }));
  const chargeAt = hud.noChargeAlert ? "off" : String(hud.chargeAlertAt || 80);

  return (
    <>
      <Surface>
        <Switch
          label="Show volume in the notch"
          description="When you change the volume or mute. Drag or scroll the bar to adjust it."
          checked={!hud.hideVolume}
          onChange={(on) => set({ hideVolume: !on })}
        />
        <Switch
          label="Show brightness in the notch"
          description="For the laptop's own screen; external monitors don't report it."
          checked={!hud.hideBrightness}
          onChange={(on) => set({ hideBrightness: !on })}
        />
        <Switch
          label="Show Caps Lock and Num Lock"
          description="A quick note when you turn them on or off."
          checked={!hud.hideCaps}
          onChange={(on) => set({ hideCaps: !on })}
        />
        <Switch
          label="Show keyboard language changes"
          description="When you switch language (Win + Space), the notch says which one you're on."
          checked={!hud.hideLanguage}
          onChange={(on) => set({ hideLanguage: !on })}
        />
      </Surface>
      <Surface>
        <Switch
          label="Show Bluetooth devices connecting"
          description="Earbuds, headphones and other devices, with their battery when they report it."
          checked={!hud.hideBluetooth}
          onChange={(on) => set({ hideBluetooth: !on })}
        />
        <Switch
          label="Warn when an accessory is low"
          description="Earbuds, mice, keyboards and Xbox controllers at 15% or less."
          checked={!hud.hideDeviceLow}
          onChange={(on) => set({ hideDeviceLow: !on })}
        />
        <Switch
          label="Internet down and back"
          description="Says when your connection drops, and when it's back."
          checked={!hud.hideInternet}
          onChange={(on) => set({ hideInternet: !on })}
        />
        <Switch
          label="Show charging"
          description="When you plug in: the battery level and how long until it's full."
          checked={!hud.hideCharging}
          onChange={(on) => set({ hideCharging: !on })}
        />
        <Row
          label="Remind me to unplug"
          description="Keeping a laptop at 100% wears the battery faster. Get a reminder when it reaches:"
        >
          <Select
            label="Remind me to unplug"
            value={chargeAt}
            options={[
              { value: "off", label: "Never" },
              { value: "80", label: "80%" },
              { value: "85", label: "85%" },
              { value: "90", label: "90%" },
              { value: "100", label: "Fully charged" },
            ]}
            onChange={(v) =>
              set(v === "off" ? { noChargeAlert: true } : { noChargeAlert: false, chargeAlertAt: Number(v) })
            }
          />
        </Row>
        <Row
          label="Monthly data cap"
          description="For metered or mobile data: a warning at 80% and 100% of what this PC uses in a month."
        >
          <Select
            label="Monthly data cap"
            value={String(hud.dataCapGb)}
            options={[
              { value: "0", label: "No cap" },
              ...[10, 25, 50, 100, 200, 500, 1000].map((gb) => ({ value: String(gb), label: `${gb} GB` })),
            ]}
            onChange={(v) => set({ dataCapGb: Number(v) })}
          />
        </Row>
      </Surface>
    </>
  );
}
