import type { MusicSettings } from "../core/generated/MusicSettings";
import { Surface, Switch } from "../design/components";
import { useSettings } from ".";

/** Settings → Media: music-reactive notch and lyrics. */
export function MusicSetting() {
  const music = useSettings((s) => s.settings?.music);
  if (!music) return null;
  const set = (change: Partial<MusicSettings>) =>
    void useSettings.getState().update((s) => ({ ...s, music: { ...s.music, ...change } }));
  return (
    <Surface>
      <Switch
        label="React to music"
        description="While music plays, the notch glows in the album's colors and pulses with the beat. Only the loudness is read — nothing is recorded."
        checked={!music.hideReactive}
        onChange={(on) => set({ hideReactive: !on })}
      />
      <Switch
        label="Show lyrics"
        description="Time-synced lyrics in Media, from LRCLIB. This sends the song's title and artist to lrclib.net."
        checked={!music.hideLyrics}
        onChange={(on) => set({ hideLyrics: !on })}
      />
    </Surface>
  );
}
