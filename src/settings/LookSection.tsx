import { Music2 } from "lucide-react";
import type { CSSProperties } from "react";

import type { AppearanceSettings } from "../core/generated/AppearanceSettings";
import type { BubbleStyle } from "../core/generated/BubbleStyle";
import type { Scale } from "../core/generated/Scale";
import type { Theme } from "../core/generated/Theme";
import { SegmentedControl, Surface, Switch } from "../design/components";
import { appearanceVars, scaledRadius, SIZE_SCALE } from "../shell/appearance";
import { useSettings } from ".";
import { ColorChoice, Row, Slider } from "./controls";
import { patch, resetGroups } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

const SCALES: { value: Scale; label: string }[] = [
  { value: "small", label: "Small" },
  { value: "normal", label: "Normal" },
  { value: "large", label: "Large" },
];

/** A still copy of the notch: the resting bubble and a notification, in the chosen look. */
function Preview({ look }: { look: AppearanceSettings }) {
  const vars = appearanceVars(look) as CSSProperties;
  const zoom = SIZE_SCALE[look.size];
  const bubble =
    look.bubble === "dot"
      ? { width: 22, height: 12 }
      : look.bubbleClock && look.bubble === "pill"
        ? { width: 104, height: 18 }
        : { width: 64, height: 14 };
  return (
    <div className="look-preview" style={vars} aria-label="Preview of the notch" role="img">
      <div className="look-preview__stage" style={{ zoom }}>
        <div
          className={`look-preview__bubble${look.bubble === "hidden" ? " look-preview__bubble--hidden" : ""}`}
          style={{ ...bubble, borderRadius: scaledRadius(bubble.height / 2, look.roundness) }}
        >
          {look.bubbleClock && look.bubble === "pill" && <span className="look-preview__clock">6:12</span>}
        </div>
        <div className="look-preview__card" style={{ borderRadius: scaledRadius(36, look.roundness) }}>
          <span className="look-preview__art">
            <Music2 size={16} aria-hidden />
          </span>
          <span className="look-preview__text">
            <span className="look-preview__title">Now playing</span>
            <span className="look-preview__body">Blinding Lights · The Weeknd</span>
          </span>
          <span className="look-preview__button">Open</span>
        </div>
      </div>
      {look.bubble === "hidden" && (
        <p className="look-preview__note">The bubble appears when you point at it.</p>
      )}
    </div>
  );
}

export function LookSection() {
  const look = useSettings((s) => s.settings?.appearance);
  const music = useSettings((s) => s.settings?.music);
  if (!look || !music) return null;
  const set = (change: Partial<AppearanceSettings>) => patch("appearance", change);

  return (
    <Section title="Look" lead={pageLead("look")} onReset={() => void resetGroups(["appearance"])}>
      <Preview look={look} />
      <Surface>
        <Row label="Notch size" description="Everything in the notch gets bigger or smaller.">
          <SegmentedControl<Scale>
            label="Notch size"
            value={look.size}
            options={SCALES}
            onChange={(size) => set({ size })}
          />
        </Row>
        <Row label="Text size" description="Just the words, if they're hard to read.">
          <SegmentedControl<Scale>
            label="Text size"
            value={look.textSize}
            options={SCALES}
            onChange={(textSize) => set({ textSize })}
          />
        </Row>
      </Surface>
      <Surface>
        <Row
          label="Resting bubble"
          description="What sits at the top of the screen when nothing is happening."
        >
          <SegmentedControl<BubbleStyle>
            label="Resting bubble"
            value={look.bubble}
            options={[
              { value: "pill", label: "Pill" },
              { value: "dot", label: "Dot" },
              { value: "hidden", label: "Hidden" },
            ]}
            onChange={(bubble) => set({ bubble })}
          />
        </Row>
        <div id="setting-clock-in-the-bubble">
          <Switch
            label="Clock in the bubble"
            description="Shows the time in the pill. Not shown with the dot."
            checked={look.bubbleClock}
            disabled={look.bubble !== "pill"}
            onChange={(bubbleClock) => set({ bubbleClock })}
          />
        </div>
      </Surface>
      <Surface>
        <Row label="Theme" description="Glass lets the desktop show through.">
          <SegmentedControl<Theme>
            label="Theme"
            value={look.theme}
            options={[
              { value: "black", label: "Black" },
              { value: "graphite", label: "Graphite" },
              { value: "glass", label: "Glass" },
            ]}
            onChange={(theme) => set({ theme })}
          />
        </Row>
        <Row label="Opacity" description="Lower lets more of what's behind show through.">
          <Slider
            label="Opacity"
            value={look.opacity}
            min={40}
            max={100}
            onChange={(opacity) => set({ opacity })}
            format={(v) => `${v}%`}
          />
        </Row>
        <Row label="Corner roundness" description="From soft corners to fully round.">
          <Slider
            label="Corner roundness"
            value={look.roundness}
            min={0}
            max={100}
            onChange={(roundness) => set({ roundness })}
            format={(v) => `${v}%`}
          />
        </Row>
      </Surface>
      <Surface>
        <Row label="Accent color" description="Buttons, switches and highlights." stacked>
          <ColorChoice value={look.accent} onChange={(accent) => set({ accent })} />
        </Row>
        <div id="setting-match-the-music">
          <Switch
            label="Match the music"
            description="While music plays, highlights take the album cover's color."
            checked={look.accentFromMusic}
            onChange={(accentFromMusic) => set({ accentFromMusic })}
          />
        </div>
        <div id="setting-music-glow">
          <Switch
            label="Music glow"
            description="While music plays, a colored outline around the notch (blue when there's no album art) that brightens on the beat. Also in Settings → Media."
            checked={!music.hideBorder && !music.hideReactive}
            onChange={(on) =>
              patch("music", on ? { hideBorder: false, hideReactive: false } : { hideBorder: true })
            }
          />
        </div>
      </Surface>
      <Surface>
        <Row
          label="Move left or right"
          description="Nudge the notch away from the center, e.g. past a webcam."
        >
          <Slider
            label="Move left or right"
            value={look.offsetX}
            min={-600}
            max={600}
            step={10}
            onChange={(offsetX) => set({ offsetX })}
            format={(v) => (v === 0 ? "Center" : `${Math.abs(v)} ${v < 0 ? "left" : "right"}`)}
          />
        </Row>
        <Row label="Distance from the top" description="Leave a gap above the notch.">
          <Slider
            label="Distance from the top"
            value={look.offsetY}
            min={0}
            max={40}
            onChange={(offsetY) => set({ offsetY })}
            format={(v) => (v === 0 ? "None" : `${v} px`)}
          />
        </Row>
      </Surface>
    </Section>
  );
}
