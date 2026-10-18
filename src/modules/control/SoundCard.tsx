// Control → Sound: where sound comes out (click a device to switch) and each app's own volume.
import { ArrowLeft, Volume2, VolumeX } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { Mixer } from "../../core/generated/Mixer";
import { ipc, isIpcError } from "../../core/ipc";
import { IconButton } from "../../design/components";

const REFRESH_MS = 4000;
/** Dragging a slider sends at most this often. */
const SEND_EVERY_MS = 60;

export function SoundCard() {
  const [mixer, setMixer] = useState<Mixer | null>(null);
  const [error, setError] = useState<string | null>(null);
  const dragging = useRef<number | null>(null);
  const lastSent = useRef(0);

  useEffect(() => {
    let live = true;
    const read = () =>
      ipc.audioMixer().then(
        (m) => {
          // Don't jump a slider you're dragging.
          if (live && dragging.current === null) setMixer(m);
        },
        (e) => live && setError(isIpcError(e) ? e.message : "Couldn't read the mixer"),
      );
    void read();
    const id = window.setInterval(() => void read(), REFRESH_MS);
    return () => {
      live = false;
      window.clearInterval(id);
    };
  }, []);

  if (!mixer) return error ? <p className="control-error">{error}</p> : null;

  const setLevel = (pid: number, level: number) => {
    setMixer((m) => m && { ...m, apps: m.apps.map((a) => (a.pid === pid ? { ...a, level } : a)) });
    const now = performance.now();
    if (now - lastSent.current < SEND_EVERY_MS) return;
    lastSent.current = now;
    void ipc.audioSetAppVolume(pid, level).catch(() => {});
  };

  return (
    <div className="control-card sound-card">
      {mixer.devices.length > 1 && (
        <div className="sound-card__devices" role="radiogroup" aria-label="Sound output">
          {mixer.devices.map((d) => (
            <button
              key={d.id}
              type="button"
              role="radio"
              aria-checked={d.default}
              className={`sound-card__device${d.default ? " sound-card__device--on" : ""}`}
              title={d.name}
              onClick={() =>
                void ipc.audioSetOutput(d.id).then(
                  () => ipc.audioMixer().then(setMixer),
                  (e) => setError(isIpcError(e) ? e.message : "Couldn't switch"),
                )
              }
            >
              {d.name}
            </button>
          ))}
        </div>
      )}
      {mixer.apps.length === 0 ? (
        <p className="control-card__sub">No apps are playing sound.</p>
      ) : (
        <ul className="sound-card__apps" aria-label="App volumes">
          {mixer.apps.map((a) => (
            <li key={a.pid} className={`sound-card__app${a.active ? "" : " sound-card__app--quiet"}`}>
              <IconButton
                icon={a.muted ? VolumeX : Volume2}
                label={a.muted ? `Unmute ${a.name}` : `Mute ${a.name}`}
                size="sm"
                active={a.muted}
                onClick={() =>
                  void ipc.audioSetAppMute(a.pid, !a.muted).then(() => ipc.audioMixer().then(setMixer))
                }
              />
              <span className="sound-card__name" title={a.name}>
                {a.name}
              </span>
              <input
                type="range"
                min={0}
                max={100}
                value={Math.round(a.level * 100)}
                aria-label={`${a.name} volume`}
                onPointerDown={() => (dragging.current = a.pid)}
                onPointerUp={() => {
                  dragging.current = null;
                  void ipc.audioSetAppVolume(a.pid, a.level).catch(() => {});
                }}
                onChange={(e) => setLevel(a.pid, Number(e.target.value) / 100)}
              />
              <span className="sound-card__value">{Math.round(a.level * 100)}</span>
            </li>
          ))}
        </ul>
      )}
      {error && <p className="control-error">{error}</p>}
    </div>
  );
}

/** The Sound view in Control, with a way back. */
export function SoundPanel({ onBack }: { onBack: () => void }) {
  return (
    <section className="control network" aria-label="Sound">
      <div className="network__bar">
        <IconButton icon={ArrowLeft} label="Back" size="sm" onClick={onBack} />
        <span className="network__heading">Sound</span>
      </div>
      <SoundCard />
    </section>
  );
}
