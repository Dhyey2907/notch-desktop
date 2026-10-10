// Settings → Pet: who your pet is, how it talks, what it reacts to, where it shows, and its stats.
import { useState } from "react";

import type { PetSettings } from "../core/generated/PetSettings";
import { Button, ConfirmButton, SegmentedControl, Surface, Switch, TextField } from "../design/components";
import { lineFor, TONES, type LineEvent } from "../pet/lines";
import { PetFigure } from "../pet/PetFigure";
import { useSettings } from ".";
import { ColorChoice, Row } from "./controls";
import { patch, resetGroups } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";
import "./petSettings.css";

const SAMPLES: LineEvent[] = ["focusDone", "meeting", "breakDone", "batteryLow", "late", "pat"];
const SAMPLE_VARS = { minutes: 5, title: "Standup", percent: 15, days: 4, agent: "Claude Code" };

const TALK_TOPICS: { key: keyof PetSettings; label: string; description: string }[] = [
  {
    key: "sayGreeting",
    label: "Morning hello",
    description: "Says hi the first time you sit down each day.",
  },
  { key: "sayMeetings", label: "Meetings", description: "A heads-up 5 minutes before a meeting." },
  {
    key: "sayBreaks",
    label: "Breaks and streaks",
    description: "Cheers when you take a break or keep a streak going.",
  },
  { key: "sayFocus", label: "Focus sessions", description: "Celebrates a finished focus session." },
  {
    key: "sayAgents",
    label: "Coding agents",
    description: "Notes when Claude Code, Codex or Gemini finishes or needs you.",
  },
  {
    key: "sayBattery",
    label: "Battery",
    description: "Mentions a low battery, and thanks you for plugging in.",
  },
  { key: "sayWeather", label: "Weather", description: "Comments when it starts raining." },
  {
    key: "sayLate",
    label: "Late nights",
    description: "Nudges you to rest when you're up between 1 and 5 am.",
  },
];

const REACTIONS: { key: keyof PetSettings; label: string; description: string }[] = [
  {
    key: "reactMusic",
    label: "Dances to music",
    description: "Wears headphones and moves to the beat while music plays.",
  },
  {
    key: "reactVideo",
    label: "Watches with you",
    description: "Grabs popcorn during Netflix, Prime Video, YouTube and other shows.",
  },
  {
    key: "reactCoding",
    label: "Coding buddy",
    description: "Types on a tiny laptop while an agent works, and gets nervous when one waits for you.",
  },
  {
    key: "reactWeather",
    label: "Dresses for the weather",
    description: "An umbrella in the rain, sunglasses when it's hot, a scarf when it's cold.",
  },
  {
    key: "reactBattery",
    label: "Battery mood",
    description: "Gets droopy when your battery is low and you're not plugged in.",
  },
  {
    key: "reactDay",
    label: "Moods from your day",
    description: "Looks stressed on busy days and yawns late at night.",
  },
];

const LEAVE = [
  { value: "click", label: "When I click the screen" },
  { value: "manual", label: "When I send it back" },
];

const CHAT_EVERY = [
  { value: "0", label: "Off" },
  { value: "2", label: "2 min" },
  { value: "5", label: "5 min" },
];

const CHATTINESS = [
  { value: "quiet", label: "Quiet" },
  { value: "normal", label: "Normal" },
  { value: "chatty", label: "Chatty" },
];

export function PetSection() {
  const pet = useSettings((s) => s.settings?.pet);
  const focus = useSettings((s) => s.settings?.focus);
  const fun = useSettings((s) => s.settings?.fun);
  const [sample, setSample] = useState(0);
  if (!pet || !focus || !fun) return null;
  const set = (change: Partial<PetSettings>) => patch("pet", change);
  const name = pet.name.trim() || "Pet";
  const event = SAMPLES[sample % SAMPLES.length];

  return (
    <Section title="Pet" lead={pageLead("pet")}>
      <Surface>
        <div className="petset__hero">
          <PetFigure size={84} face="happy" prop="none" outfit="none" motion="idle" color={pet.color} />
          <div className="petset__hero-text">
            <NameField value={pet.name} onSave={(n) => set({ name: n })} />
            <Row label="Color" stacked>
              <ColorChoice value={pet.color} onChange={(color) => set({ color })} />
            </Row>
          </div>
        </div>
      </Surface>

      <h2 id="setting-personality" className="settings__subhead">
        Personality
      </h2>
      <p className="settings__lead">How {name} talks to you. Every line it says follows this tone.</p>
      <div className="petset__tones" role="radiogroup" aria-label="Tone">
        {TONES.map((t) => (
          <button
            key={t.id}
            type="button"
            role="radio"
            aria-checked={pet.tone === t.id}
            className={`petset__tone${pet.tone === t.id ? " petset__tone--on" : ""}`}
            onClick={() => set({ tone: t.id })}
          >
            <span className="petset__tone-name">{t.label}</span>
            <span className="petset__tone-blurb">{t.blurb}</span>
            <span className="petset__tone-sample">
              “{lineFor(t.id, event, { ...SAMPLE_VARS, name }, () => 0)}”
            </span>
          </button>
        ))}
      </div>
      <div className="settings__buttons">
        <Button onClick={() => setSample(sample + 1)}>Show another example</Button>
      </div>

      <h2 id="setting-talking" className="settings__subhead">
        Talking
      </h2>
      <Surface>
        <Switch
          label="Talks to you"
          description="Short lines in a small card at the right moments. Quiet hours, focus sessions and games keep it quiet (except meeting heads-ups)."
          checked={pet.talk}
          onChange={(talk) => set({ talk })}
        />
        {pet.talk && (
          <>
            <Row
              label="How often"
              description="Quiet: at most every 45 min · Normal: 15 min · Chatty: 4 min."
            >
              <SegmentedControl<string>
                label="How often"
                value={pet.chattiness}
                options={CHATTINESS}
                onChange={(chattiness) => set({ chattiness })}
              />
            </Row>
            <Row
              label="Says something on the taskbar"
              description={`While you keep ${name} out on the taskbar, it says something in its cloud this often. Reminders that come due show there too.`}
            >
              <SegmentedControl<string>
                label="Says something on the taskbar"
                value={pet.chatMinutes === 0 ? "0" : pet.chatMinutes <= 2 ? "2" : "5"}
                options={CHAT_EVERY}
                onChange={(v) => set({ chatMinutes: Number(v) })}
              />
            </Row>
            {TALK_TOPICS.map((t) => (
              <Switch
                key={t.key}
                label={t.label}
                description={t.description}
                checked={pet[t.key] as boolean}
                onChange={(on) => set({ [t.key]: on })}
              />
            ))}
          </>
        )}
      </Surface>

      <h2 id="setting-reactions" className="settings__subhead">
        Reactions
      </h2>
      <Surface>
        {REACTIONS.map((r) => (
          <Switch
            key={r.key}
            label={r.label}
            description={r.description}
            checked={pet[r.key] as boolean}
            onChange={(on) => set({ [r.key]: on })}
          />
        ))}
        <Switch
          label="Petting"
          description={`Click ${name} in its section of the notch to pat it.`}
          checked={pet.petting}
          onChange={(petting) => set({ petting })}
        />
      </Surface>

      <h2 id="setting-where-it-shows" className="settings__subhead">
        Where it shows
      </h2>
      <p className="settings__lead">
        There's one {name}. It lives in the notch and goes for walks along the taskbar, where it talks through
        a little cloud. Call it out from its section of the notch and it keeps walking until you send it back;
        click it there to give it a treat. Only its own screen matters to it: what you do on another screen
        doesn't affect it.
      </p>
      <Surface>
        <Switch
          label="In the resting bubble"
          description="A tiny version next to the other marks (needs the Pill bubble style in Look)."
          checked={!focus.hidePet}
          onChange={(on) => patch("focus", { hidePet: !on })}
        />
        <Switch
          label="Walks the taskbar when you're away"
          description={
            pet.leave === "manual"
              ? "After two minutes away it strolls along the taskbar, and keeps strolling until you send it back."
              : "After two minutes away it strolls along the taskbar; click anywhere and it jumps and runs off."
          }
          checked={fun.petWalk}
          onChange={(on) => patch("fun", { petWalk: on })}
        />
        <Row
          label="Goes back after an away walk"
          description={
            pet.leave === "manual"
              ? `After a walk while you were away, ${name} keeps walking until you press Send back in its section of the notch.`
              : `After a walk while you were away, a click on ${name}'s screen sends it back into the notch.`
          }
          stacked
        >
          <SegmentedControl<string>
            label="Goes back"
            value={pet.leave === "manual" ? "manual" : "click"}
            options={LEAVE}
            onChange={(leave) => set({ leave })}
          />
        </Row>
      </Surface>

      <h2 id="setting-pet-stats" className="settings__subhead">
        Stats
      </h2>
      <Surface>
        <p className="settings__lead">
          {name} has {pet.xp.toLocaleString()} xp and {pet.pats.toLocaleString()} pats
          {pet.born ? `, and was born on ${new Date(pet.born + "T12:00").toLocaleDateString()}` : ""}.
        </p>
        <div className="settings__buttons">
          <ConfirmButton
            variant="danger"
            confirmLabel={`Start ${name} over from level 1?`}
            onConfirm={() => set({ xp: 0, pats: 0, born: "" })}
          >
            Reset stats
          </ConfirmButton>
          <ConfirmButton
            confirmLabel="Put every pet setting back to how it started?"
            onConfirm={() => void resetGroups(["pet"])}
          >
            Reset all pet settings
          </ConfirmButton>
        </div>
      </Surface>
    </Section>
  );
}

function NameField({ value, onSave }: { value: string; onSave: (name: string) => void }) {
  const [draft, setDraft] = useState(value);
  const changed = draft.trim() !== value.trim();
  return (
    <div className="settings__field">
      <TextField label="Name" value={draft} placeholder="Pet" onChange={(v) => setDraft(v.slice(0, 24))} />
      {changed && (
        <Button variant="primary" onClick={() => onSave(draft.trim())}>
          Save name
        </Button>
      )}
    </div>
  );
}
