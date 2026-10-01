import { SegmentedControl, Surface, Switch } from "../design/components";
import { useSettings } from ".";
import { Row } from "./controls";
import { patch, resetGroups } from "./patch";
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
  if (!home) return null;
  const toggle = (id: string, on: boolean) =>
    patch("home", {
      hiddenTiles: on ? home.hiddenTiles.filter((t) => t !== id) : [...home.hiddenTiles, id],
    });

  return (
    <Section title="Home" lead={pageLead("home")} onReset={() => void resetGroups(["home"])}>
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
    </Section>
  );
}
