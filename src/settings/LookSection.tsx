import { Music2 } from "lucide-react";
import type { CSSProperties } from "react";

import type { AppearanceSettings } from "../core/generated/AppearanceSettings";
import type { BubbleStyle } from "../core/generated/BubbleStyle";
import type { ScreenEdge } from "../core/generated/ScreenEdge";
import type { Scale } from "../core/generated/Scale";
import type { Theme } from "../core/generated/Theme";
import { SegmentedControl, Surface, Switch } from "../design/components";
import { appearanceVars, scaledRadius, SIZE_SCALE } from "../shell/appearance";
import { ipc } from "../core/ipc";
import { useSettings } from ".";
import { ColorChoice, Row, Select, Slider } from "./controls";
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
  const away = useSettings((s) => s.settings?.away);
  const music = useSettings((s) => s.settings?.music);
  if (!look || !music) return null;
  const set = (change: Partial<AppearanceSettings>) => patch("appearance", change);
  const side = look.edge === "left" || look.edge === "right";

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
        <div id="setting-match-the-wallpaper">
          <Switch
            label="Match the wallpaper"
            description="Highlights take a color from your desktop wallpaper, and change with it."
            checked={look.accentFromWallpaper}
            onChange={(accentFromWallpaper) => set({ accentFromWallpaper })}
          />
        </div>
        <div id="setting-music-glow">
          <Switch
            label="Music glow"
            description="While something plays, the notch glows in the player's colors (the album art's, or Netflix red, Prime blue…) and brightens on the beat. Also in Settings → Media."
            checked={!music.hideBorder && !music.hideReactive}
            onChange={(on) =>
              patch("music", on ? { hideBorder: false, hideReactive: false } : { hideBorder: true })
            }
          />
        </div>
      </Surface>
      <Surface>
        <div id="setting-screen-edge">
          <Row
            label="Screen edge"
            description="Top, just above the taskbar, or either side. Or hold Alt and drag the resting notch to any edge or screen."
          >
            <SegmentedControl<ScreenEdge>
              label="Screen edge"
              value={look.edge}
              options={[
                { value: "top", label: "Top" },
                { value: "bottom", label: "Bottom" },
                { value: "left", label: "Left" },
                { value: "right", label: "Right" },
              ]}
              onChange={(edge) => set({ edge })}
            />
          </Row>
        </div>
        {side ? (
          <Row
            label="Height on the side"
            description="How far down the screen. Or hold Alt and drag the resting notch."
          >
            <Slider
              label="Height on the side"
              value={Math.min(1200, look.offsetSide)}
              min={0}
              max={1200}
              step={10}
              onChange={(offsetSide) => set({ offsetSide })}
              format={(v) => (v === 0 ? "Top" : `${v} px down`)}
            />
          </Row>
        ) : (
          <>
            <div id="setting-notch-position">
              <Row label="Position along the edge" description="Or hold Alt and drag the resting notch.">
                <SegmentedControl<"left" | "center" | "right" | "custom">
                  label="Position along the edge"
                  value={
                    look.offsetX <= -4000
                      ? "left"
                      : look.offsetX >= 4000
                        ? "right"
                        : look.offsetX === 0
                          ? "center"
                          : "custom"
                  }
                  options={[
                    { value: "left", label: "Left" },
                    { value: "center", label: "Center" },
                    { value: "right", label: "Right" },
                    ...(look.offsetX !== 0 && Math.abs(look.offsetX) < 4000
                      ? [{ value: "custom" as const, label: "Custom" }]
                      : []),
                  ]}
                  onChange={(v) => {
                    if (v !== "custom") set({ offsetX: v === "left" ? -4000 : v === "right" ? 4000 : 0 });
                  }}
                />
              </Row>
            </div>
            <Row
              label="Move left or right"
              description="Nudge the notch away from the center, e.g. past a webcam."
            >
              <Slider
                label="Move left or right"
                value={Math.max(-600, Math.min(600, look.offsetX))}
                min={-600}
                max={600}
                step={10}
                onChange={(offsetX) => set({ offsetX })}
                format={(v) => (v === 0 ? "Center" : `${Math.abs(v)} ${v < 0 ? "left" : "right"}`)}
              />
            </Row>
          </>
        )}
        <Row
          label={
            look.edge === "bottom"
              ? "Distance from the taskbar"
              : side
                ? "Distance from the side"
                : "Distance from the top"
          }
          description={
            look.edge === "bottom"
              ? "Leave a gap below the notch."
              : side
                ? "Leave a gap beside the notch."
                : "Leave a gap above the notch."
          }
        >
          <Slider
            label="Distance from the edge"
            value={look.offsetY}
            min={0}
            max={40}
            onChange={(offsetY) => set({ offsetY })}
            format={(v) => (v === 0 ? "None" : `${v} px`)}
          />
        </Row>
      </Surface>
      <Surface>
        <div id="setting-fade-while-typing">
          <Switch
            label="Fade while typing under it"
            description="Typing in a box right under the notch (an address bar, a chat box) fades the resting bubble until you stop."
            checked={look.fadeWhileTyping}
            onChange={(fadeWhileTyping) => set({ fadeWhileTyping })}
          />
        </div>
        <div id="setting-away-screen">
          <Row
            label="Away screen"
            description="While you're away, your other monitors (or all of them) show a big clock, next meeting, weather, music and unread mail. Any key or mouse movement brings your desktop back. Also “Away screen” in the palette."
          >
            <Select
              label="Away screen"
              value={String(away?.afterIdleMin ?? 0)}
              options={[
                { value: "0", label: "Only from the palette" },
                ...[2, 5, 10, 15, 30].map((m) => ({ value: String(m), label: `After ${m} min away` })),
              ]}
              onChange={(v) =>
                void useSettings
                  .getState()
                  .update((st) => ({ ...st, away: { ...st.away, afterIdleMin: Number(v) } }))
              }
            />
          </Row>
          <Switch
            label="Away screen on every monitor"
            description="Off: only on monitors other than the main one (with one monitor, it uses that one)."
            checked={away?.allScreens ?? false}
            onChange={(allScreens) =>
              void useSettings.getState().update((st) => ({ ...st, away: { ...st.away, allScreens } }))
            }
          />
        </div>
        <div id="setting-lock-screen-info">
          <Switch
            label="Lock-screen info"
            description="Your next meeting, the weather, what's playing and unread mail, drawn onto your wallpaper as the lock-screen picture (updated every few minutes). With Windows Spotlight on, turning this off opens the lock-screen settings to pick it again."
            checked={look.lockscreenInfo}
            onChange={(lockscreenInfo) => {
              set({ lockscreenInfo });
              if (!lockscreenInfo) void ipc.lockscreenOff().catch(() => {});
            }}
          />
        </div>
      </Surface>
    </Section>
  );
}
