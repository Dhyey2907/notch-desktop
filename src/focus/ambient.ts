// Ambient sounds, synthesized live with Web Audio — no files. Each sound is noise shaped by
// filters and slow modulation; levels ramp smoothly when changed.

export const SOUNDS = ["rain", "wind", "ocean", "fire", "brown", "pink"] as const;
export type Sound = (typeof SOUNDS)[number];
export type Levels = Record<Sound, number>;

const SECONDS = 4;
/** Level changes glide over about this long. */
const RAMP = 0.25;

function noiseBuffer(ctx: AudioContext, kind: "pink" | "brown"): AudioBuffer {
  const buffer = ctx.createBuffer(1, ctx.sampleRate * SECONDS, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  let b0 = 0,
    b1 = 0,
    b2 = 0,
    b3 = 0,
    b4 = 0,
    b5 = 0,
    b6 = 0,
    last = 0;
  for (let i = 0; i < data.length; i++) {
    const white = Math.random() * 2 - 1;
    if (kind === "pink") {
      // Paul Kellet's pink filter.
      b0 = 0.99886 * b0 + white * 0.0555179;
      b1 = 0.99332 * b1 + white * 0.0750759;
      b2 = 0.969 * b2 + white * 0.153852;
      b3 = 0.8665 * b3 + white * 0.3104856;
      b4 = 0.55 * b4 + white * 0.5329522;
      b5 = -0.7616 * b5 - white * 0.016898;
      data[i] = (b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362) * 0.11;
      b6 = white * 0.115926;
    } else {
      last = (last + 0.02 * white) / 1.02;
      data[i] = last * 3.5;
    }
  }
  return buffer;
}

/** Sparse short ticks (raindrops, fire crackles) over a loop. */
function ticksBuffer(ctx: AudioContext, perSecond: number, decayMs: number): AudioBuffer {
  const buffer = ctx.createBuffer(1, ctx.sampleRate * SECONDS, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  const decay = (decayMs / 1000) * ctx.sampleRate;
  for (let n = 0; n < perSecond * SECONDS; n++) {
    const start = Math.floor(Math.random() * data.length);
    const amp = 0.2 + Math.random() * 0.8;
    for (let i = 0; i < decay * 4 && start + i < data.length; i++) {
      data[start + i] += (Math.random() * 2 - 1) * amp * Math.exp(-i / decay);
    }
  }
  return buffer;
}

function loop(ctx: AudioContext, buffer: AudioBuffer): AudioBufferSourceNode {
  const source = ctx.createBufferSource();
  source.buffer = buffer;
  source.loop = true;
  // Start each loop at a random point so layered loops don't line up.
  source.start(0, Math.random() * SECONDS);
  return source;
}

function filter(ctx: AudioContext, type: BiquadFilterType, frequency: number, q = 0.7): BiquadFilterNode {
  const f = ctx.createBiquadFilter();
  f.type = type;
  f.frequency.value = frequency;
  f.Q.value = q;
  return f;
}

/** A slow wobble added to `param` (e.g. gain or a filter's frequency). */
function lfo(ctx: AudioContext, param: AudioParam, rate: number, depth: number) {
  const osc = ctx.createOscillator();
  osc.frequency.value = rate;
  const amount = ctx.createGain();
  amount.gain.value = depth;
  osc.connect(amount).connect(param);
  osc.start();
}

/** Build one sound; returns its output (a gain node at level 0). */
function build(ctx: AudioContext, sound: Sound): GainNode {
  const out = ctx.createGain();
  out.gain.value = 0;
  const brown = () => noiseBuffer(ctx, "brown");
  switch (sound) {
    case "rain": {
      loop(ctx, noiseBuffer(ctx, "pink"))
        .connect(filter(ctx, "highpass", 500))
        .connect(filter(ctx, "lowpass", 7000))
        .connect(out);
      const drops = ctx.createGain();
      drops.gain.value = 0.5;
      loop(ctx, ticksBuffer(ctx, 40, 3))
        .connect(filter(ctx, "bandpass", 3000, 0.8))
        .connect(drops)
        .connect(out);
      break;
    }
    case "wind": {
      const lp = filter(ctx, "lowpass", 420, 1.2);
      const swell = ctx.createGain();
      swell.gain.value = 0.7;
      loop(ctx, brown()).connect(lp).connect(swell).connect(out);
      lfo(ctx, swell.gain, 0.07, 0.3);
      lfo(ctx, lp.frequency, 0.05, 180);
      break;
    }
    case "ocean": {
      const lp = filter(ctx, "lowpass", 900);
      const wave = ctx.createGain();
      wave.gain.value = 0.55;
      loop(ctx, brown()).connect(lp).connect(wave).connect(out);
      lfo(ctx, wave.gain, 0.1, 0.45);
      lfo(ctx, lp.frequency, 0.1, 400);
      break;
    }
    case "fire": {
      const base = ctx.createGain();
      base.gain.value = 0.6;
      loop(ctx, brown())
        .connect(filter(ctx, "lowpass", 260))
        .connect(base)
        .connect(out);
      const crackle = ctx.createGain();
      crackle.gain.value = 0.35;
      loop(ctx, ticksBuffer(ctx, 9, 1.2))
        .connect(filter(ctx, "highpass", 1800))
        .connect(crackle)
        .connect(out);
      break;
    }
    case "brown":
      loop(ctx, brown()).connect(out);
      break;
    case "pink":
      loop(ctx, noiseBuffer(ctx, "pink")).connect(out);
      break;
  }
  return out;
}

/** Plays the mix; each sound is built the first time it's turned up. */
export class AmbientEngine {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private sounds = new Map<Sound, GainNode>();

  apply(levels: Levels, master: number, on: boolean) {
    const audible = on && SOUNDS.some((s) => levels[s] > 0);
    if (!audible && !this.ctx) return;
    if (!this.ctx) {
      this.ctx = new AudioContext();
      this.master = this.ctx.createGain();
      this.master.gain.value = 0;
      this.master.connect(this.ctx.destination);
    }
    const ctx = this.ctx;
    const t = ctx.currentTime;
    this.master!.gain.setTargetAtTime(audible ? master : 0, t, RAMP / 3);
    for (const sound of SOUNDS) {
      const level = levels[sound];
      let node = this.sounds.get(sound);
      if (!node && level > 0 && audible) {
        node = build(ctx, sound);
        node.connect(this.master!);
        this.sounds.set(sound, node);
      }
      node?.gain.setTargetAtTime(audible ? level : 0, t, RAMP / 3);
    }
    if (audible && ctx.state === "suspended") void ctx.resume();
    if (!audible) window.setTimeout(() => this.idle(), (RAMP + 0.3) * 1000);
  }

  /** Quiet: suspend the audio context so it costs nothing. */
  private idle() {
    if (this.ctx && this.master && this.master.gain.value < 0.001) void this.ctx.suspend();
  }

  close() {
    void this.ctx?.close();
    this.ctx = null;
    this.master = null;
    this.sounds.clear();
  }
}
