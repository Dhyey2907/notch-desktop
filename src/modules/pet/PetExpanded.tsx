// The Pet section: your pet up close (click to pat it), what it's up to, its latest line, and
// its stats card — age, level, pats and streaks.
import { MessageCircle, Monitor, Settings2, Undo2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { ipc } from "../../core/ipc";
import { Button, IconButton, ModuleContainer, Progress } from "../../design/components";
import { currentStreak } from "../../focus/breaks";
import { useFocus } from "../../focus/focusStore";
import { lineFor, TONES, type LineEvent } from "../../pet/lines";
import { PetFigure } from "../../pet/PetFigure";
import { ageInDays, levelFor } from "../../pet/petState";
import { lovePet, usePetLive } from "../../pet/petStore";
import { usePetLook } from "../../pet/usePetLook";
import { usePetOut } from "../../pet/usePetPlace";
import { useSettings } from "../../settings";
import "./pet.css";

const SAVE_PATS_AFTER_MS = 1500;
const CHAT: LineEvent[] = ["greeting", "focusDone", "breakDone", "rain", "late", "pat"];

export default function PetExpanded() {
  const look = usePetLook();
  const pet = useSettings((s) => s.settings?.pet);
  const breaks = useSettings((s) => s.settings?.breaks);
  const sessions = useFocus((s) => s.state?.doneToday ?? 0);
  const line = usePetLive((s) => s.line);
  const [hearts, setHearts] = useState<number[]>([]);
  const [now] = useState(() => new Date());
  const unsaved = useRef(0);
  const saveTimer = useRef<number | undefined>(undefined);
  // One pet: while it walks the taskbar it isn't here.
  const away = usePetOut();

  // Save pats that haven't been saved yet when the section closes.
  useEffect(
    () => () => {
      window.clearTimeout(saveTimer.current);
      const add = unsaved.current;
      if (add > 0)
        void useSettings.getState().update((s) => ({ ...s, pet: { ...s.pet, pats: s.pet.pats + add } }));
    },
    [],
  );

  if (!look || !pet || !breaks) return null;
  const { level, into, needed } = levelFor(pet.xp);
  const age = ageInDays(pet.born, now);
  const streak = currentStreak(breaks, now);
  const tone = TONES.find((t) => t.id === pet.tone)?.label ?? "Cheerful";

  const say = (event: LineEvent) =>
    usePetLive.setState({
      line: {
        text: lineFor(pet.tone, event, {
          name: look.name,
          minutes: 5,
          title: "Standup",
          percent: 18,
          days: 3,
        }),
        event,
        at: Date.now(),
      },
    });

  const pat = () => {
    if (!pet.petting) return;
    lovePet();
    say("pat");
    const id = Date.now();
    setHearts((h) => [...h.slice(-5), id]);
    window.setTimeout(() => setHearts((h) => h.filter((x) => x !== id)), 1100);
    unsaved.current += 1;
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      const add = unsaved.current;
      unsaved.current = 0;
      void useSettings.getState().update((s) => ({ ...s, pet: { ...s.pet, pats: s.pet.pats + add } }));
    }, SAVE_PATS_AFTER_MS);
  };

  return (
    <ModuleContainer
      title={look.name}
      trailing={
        <IconButton
          icon={Settings2}
          label="Pet settings"
          size="sm"
          onClick={() => void ipc.openSettingsWindow("pet")}
        />
      }
    >
      <div className="petsec">
        <button
          type="button"
          className="petsec__stage"
          onClick={pat}
          aria-label={away ? `${look.name} is out for a walk` : pet.petting ? `Pat ${look.name}` : look.name}
          disabled={away || !pet.petting}
        >
          {away ? (
            <span className="petsec__away">
              Out for a walk
              <small>Send back to bring {look.name} home</small>
            </span>
          ) : (
            <PetFigure
              size={112}
              face={look.face}
              prop={look.prop}
              outfit={look.outfit}
              motion={look.motion}
              color={look.color}
            />
          )}
          {hearts.map((id) => (
            <span key={id} className="petsec__heart" aria-hidden>
              ♥
            </span>
          ))}
        </button>

        <div className="petsec__side">
          <p className="petsec__status">{look.status}</p>
          <div className="petsec__bubble" aria-live="polite">
            {line ? line.text : pet.talk ? `Hi! I'm ${look.name}.` : "(Talking is off)"}
          </div>
          <div className="petsec__stats">
            <div className="petsec__stat petsec__stat--wide">
              <span className="petsec__label">Level {level}</span>
              <Progress label="Experience to the next level" value={into} max={needed} tone="info" />
              <span className="petsec__hint">
                {into}/{needed} xp · focus, breaks and tasks help it grow
              </span>
            </div>
            <Stat label="Age" value={age === 0 ? "New!" : `${age} ${age === 1 ? "day" : "days"}`} />
            <Stat
              label="Pats · treats"
              value={`${pet.pats.toLocaleString()} · ${pet.treats.toLocaleString()}`}
            />
            <Stat label="Break streak" value={String(streak)} hint={`best ${breaks.best}`} />
            <Stat label="Focus today" value={String(sessions)} />
          </div>
          <div className="petsec__actions">
            <Button onClick={() => say(CHAT[Math.floor(Math.random() * CHAT.length)])}>
              <MessageCircle size={13} aria-hidden /> Say something
            </Button>
            <Button
              variant={away ? "secondary" : "primary"}
              onClick={() => void (away ? ipc.petBuddyHide() : ipc.petBuddyShow())}
            >
              {away ? <Undo2 size={13} aria-hidden /> : <Monitor size={13} aria-hidden />}{" "}
              {away ? "Send back" : "Call out"}
            </Button>
            <span className="petsec__tone">Tone: {tone}</span>
          </div>
        </div>
      </div>
    </ModuleContainer>
  );
}

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="petsec__stat">
      <span className="petsec__label">{label}</span>
      <span className="petsec__value">
        {value}
        {hint && <span className="petsec__hint"> · {hint}</span>}
      </span>
    </div>
  );
}
