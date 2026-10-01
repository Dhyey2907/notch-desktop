// Notification sounds, made live with Web Audio (no sound files): a soft tap or a two-note chime.
import type { AlertSound } from "../core/generated/AlertSound";

let context: AudioContext | null = null;

function note(ctx: AudioContext, freq: number, start: number, length: number, volume: number) {
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = "sine";
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(0, start);
  gain.gain.linearRampToValueAtTime(volume, start + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
  osc.connect(gain).connect(ctx.destination);
  osc.start(start);
  osc.stop(start + length + 0.02);
}

export async function playAlert(sound: AlertSound, volume: number) {
  if (sound === "off" || volume <= 0) return;
  context ??= new AudioContext();
  // WebView2 may start the context suspended until the page has been clicked once.
  if (context.state === "suspended") await context.resume().catch(() => {});
  const t = context.currentTime + 0.01;
  const v = 0.25 * volume;
  if (sound === "soft") {
    note(context, 880, t, 0.18, v * 0.8);
  } else {
    note(context, 988, t, 0.35, v);
    note(context, 1319, t + 0.12, 0.5, v);
  }
}
