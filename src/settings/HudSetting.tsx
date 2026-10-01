import type { HudSettings } from "../core/generated/HudSettings";
import { Surface, Switch } from "../design/components";
import { useSettings } from ".";

/** Settings → General: which system changes pop up in the notch. */
export function HudSetting() {
  const hud = useSettings((s) => s.settings?.hud);
  if (!hud) return null;
  const set = (change: Partial<HudSettings>) =>
    void useSettings.getState().update((s) => ({ ...s, hud: { ...s.hud, ...change } }));

  return (
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
        label="Show Bluetooth devices connecting"
        description="Earbuds, headphones and other devices, with their battery when they report it."
        checked={!hud.hideBluetooth}
        onChange={(on) => set({ hideBluetooth: !on })}
      />
    </Surface>
  );
}
