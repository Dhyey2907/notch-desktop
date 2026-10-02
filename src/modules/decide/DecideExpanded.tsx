// Decision maker: flip a coin, roll dice, or spin a wheel of your own options.
import { useState } from "react";

import { Button, ModuleContainer, SegmentedControl } from "../../design/components";
import { parseOptions, randomBelow, wheelRotation } from "./decide";
import "./decide.css";

type Tab = "coin" | "dice" | "wheel";

const SPIN_MS = 1400;
const OPTIONS_KEY = "notch-decide-options";
const DEFAULT_OPTIONS = "Pizza, Sushi, Tacos, Pasta";
const WHEEL_COLORS = ["#8ab4ff", "#f7a072", "#9be28f", "#e79be8", "#f5d76e", "#7fd8d0"];

function savedOptions(): string {
  try {
    return localStorage.getItem(OPTIONS_KEY) ?? DEFAULT_OPTIONS;
  } catch {
    return DEFAULT_OPTIONS;
  }
}

export default function DecideExpanded() {
  const [tab, setTab] = useState<Tab>("coin");
  return (
    <ModuleContainer
      title="Decide"
      trailing={
        <SegmentedControl<Tab>
          label="Decision maker"
          value={tab}
          options={[
            { value: "coin", label: "Coin" },
            { value: "dice", label: "Dice" },
            { value: "wheel", label: "Wheel" },
          ]}
          onChange={setTab}
        />
      }
    >
      {tab === "coin" ? <Coin /> : tab === "dice" ? <Dice /> : <Wheel />}
    </ModuleContainer>
  );
}

function Coin() {
  const [flips, setFlips] = useState(0);
  const [side, setSide] = useState<"Heads" | "Tails" | null>(null);
  const [busy, setBusy] = useState(false);
  const flip = () => {
    const next = randomBelow(2) === 0 ? "Heads" : "Tails";
    setBusy(true);
    setSide(null);
    setFlips((f) => f + 1);
    window.setTimeout(() => {
      setSide(next);
      setBusy(false);
    }, 700);
  };
  return (
    <div className="decide">
      <div key={flips} className={`decide-coin${busy ? " decide-coin--flip" : ""}`}>
        {side ? side[0] : "?"}
      </div>
      <div className="decide-result" aria-live="polite">
        {side ?? (busy ? "Flipping…" : "Heads or tails?")}
      </div>
      <Button variant="primary" onClick={flip} disabled={busy}>
        Flip
      </Button>
    </div>
  );
}

const PIPS: Record<number, number[]> = {
  1: [4],
  2: [0, 8],
  3: [0, 4, 8],
  4: [0, 2, 6, 8],
  5: [0, 2, 4, 6, 8],
  6: [0, 2, 3, 5, 6, 8],
};

function Die({ value, rolling }: { value: number; rolling: boolean }) {
  return (
    <div className={`decide-die${rolling ? " decide-die--roll" : ""}`} role="img" aria-label={`${value}`}>
      {Array.from({ length: 9 }, (_, i) => (
        <span key={i} className={PIPS[value].includes(i) ? "decide-pip" : undefined} />
      ))}
    </div>
  );
}

function Dice() {
  const [count, setCount] = useState(1);
  const [values, setValues] = useState<number[]>([6]);
  const [rolls, setRolls] = useState(0);
  const [rolling, setRolling] = useState(false);
  const roll = () => {
    const next = Array.from({ length: count }, () => randomBelow(6) + 1);
    setRolling(true);
    setRolls((r) => r + 1);
    window.setTimeout(() => {
      setValues(next);
      setRolling(false);
    }, 600);
  };
  const shown = Array.from({ length: count }, (_, i) => values[i] ?? 6);
  const total = shown.reduce((a, b) => a + b, 0);
  return (
    <div className="decide">
      <div key={rolls} className="decide-dice">
        {shown.map((v, i) => (
          <Die key={i} value={v} rolling={rolling} />
        ))}
      </div>
      <div className="decide-result" aria-live="polite">
        {rolling ? "Rolling…" : count > 1 ? `Total ${total}` : `You rolled ${shown[0]}`}
      </div>
      <div className="decide-row">
        <SegmentedControl<string>
          label="Number of dice"
          value={String(count)}
          options={[1, 2, 3].map((n) => ({ value: String(n), label: `${n}` }))}
          onChange={(v) => setCount(Number(v))}
        />
        <Button variant="primary" onClick={roll} disabled={rolling}>
          Roll
        </Button>
      </div>
    </div>
  );
}

function Wheel() {
  const [text, setText] = useState(savedOptions);
  const [rotation, setRotation] = useState(0);
  const [winner, setWinner] = useState<string | null>(null);
  const [spinning, setSpinning] = useState(false);
  const options = parseOptions(text);
  const slice = options.length > 0 ? 360 / options.length : 360;

  const save = (value: string) => {
    setText(value);
    setWinner(null);
    try {
      localStorage.setItem(OPTIONS_KEY, value);
    } catch {
      // Not saved; it still works for now.
    }
  };

  const spin = () => {
    if (options.length < 2) return;
    const index = randomBelow(options.length);
    setWinner(null);
    setSpinning(true);
    setRotation((r) => wheelRotation(r, index, options.length));
    window.setTimeout(() => {
      setWinner(options[index]);
      setSpinning(false);
    }, SPIN_MS);
  };

  const gradient = options
    .map((_, i) => `${WHEEL_COLORS[i % WHEEL_COLORS.length]} ${i * slice}deg ${(i + 1) * slice}deg`)
    .join(", ");

  return (
    <div className="decide decide--wheel">
      <div className="decide-wheel-box">
        <div className="decide-pointer" aria-hidden />
        <div
          className="decide-wheel"
          style={{
            background: options.length ? `conic-gradient(${gradient})` : "var(--surface)",
            transform: `rotate(${rotation}deg)`,
            transitionDuration: `${SPIN_MS}ms`,
          }}
          aria-hidden
        >
          {options.map((o, i) => (
            <span
              key={o}
              className="decide-label"
              style={{ transform: `rotate(${(i + 0.5) * slice}deg) translateY(-38px)` }}
            >
              {o.slice(0, 9)}
            </span>
          ))}
        </div>
      </div>
      <div className="decide-side">
        <div className="decide-result" aria-live="polite">
          {winner ??
            (spinning ? "Spinning…" : options.length < 2 ? "Add two or more options" : "Spin to decide")}
        </div>
        <textarea
          className="decide-options"
          aria-label="Options, separated by commas"
          value={text}
          rows={3}
          onChange={(e) => save(e.target.value)}
        />
        <Button variant="primary" onClick={spin} disabled={spinning || options.length < 2}>
          Spin
        </Button>
      </div>
    </div>
  );
}
