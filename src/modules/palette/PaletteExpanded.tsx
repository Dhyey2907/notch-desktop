// Type to find any command: arrows move, Enter runs it, Esc closes.
import { CornerDownLeft } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import { ipc, isIpcError } from "../../core/ipc";
import { ModuleContainer } from "../../design/components";
import type { InstalledApp } from "../../core/generated/InstalledApp";

import { appCommands, commands, search } from "./commands";
import { usePalette } from "./paletteStore";
import "./palette.css";

const SHOWN = 8;
const CONFIRM_MS = 5000;

export default function PaletteExpanded() {
  const [input, setInput] = useState("");
  const [index, setIndex] = useState(0);
  const [result, setResult] = useState<string | null>(null);
  /** A command that asked to be confirmed, and when (a second Enter within a few seconds runs it). */
  const armed = useRef<{ id: string; at: number } | null>(null);
  const [apps, setApps] = useState<InstalledApp[]>([]);
  const all = useMemo(() => [...commands(), ...appCommands(apps)], [apps]);
  const matches = useMemo(() => search(all, input).slice(0, SHOWN), [all, input]);
  const field = useRef<HTMLInputElement>(null);
  const prefillAt = usePalette((s) => s.at);

  // Installed apps for the launcher (cached on the Rust side).
  useEffect(() => {
    let live = true;
    ipc.appsList().then(
      (list) => live && setApps(list),
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);

  // The quick-note shortcut opens it with "note " typed.
  useEffect(() => {
    if (!prefillAt) return;
    const text = usePalette.getState().prefill;
    const id = window.setTimeout(() => {
      setInput(text);
      setIndex(0);
      field.current?.focus();
    }, 0);
    return () => window.clearTimeout(id);
  }, [prefillAt]);

  const run = async (i: number) => {
    const pick = matches[i];
    if (!pick) return;
    const { confirm, id } = pick.command;
    if (confirm) {
      const a = armed.current;
      if (!a || a.id !== id || Date.now() - a.at > CONFIRM_MS) {
        armed.current = { id, at: Date.now() };
        setResult(confirm);
        return;
      }
      armed.current = null;
    }
    try {
      const out = await pick.command.run(pick.arg);
      setResult(out ?? `Done: ${pick.command.title}`);
      setInput("");
      setIndex(0);
    } catch (error) {
      setResult(isIpcError(error) ? error.message : "That didn't work");
    }
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      e.stopPropagation();
      const n = matches.length || 1;
      setIndex((i) => (i + (e.key === "ArrowDown" ? 1 : -1) + n) % n);
    } else if (e.key === "Enter") {
      e.preventDefault();
      void run(index);
    } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      // Stay in the text box (the dock would switch modules).
      e.stopPropagation();
    }
  };

  return (
    <ModuleContainer title="Commands">
      <div className="palette">
        <input
          ref={field}
          className="palette__input"
          autoFocus
          aria-label="Type a command"
          placeholder="Type a command — “timer 10”, “note buy milk”, “open mail”…"
          value={input}
          onChange={(e) => {
            setInput(e.target.value);
            setIndex(0);
            setResult(null);
          }}
          onKeyDown={onKeyDown}
        />
        {result && (
          <p className="palette__result" role="status">
            {result}
          </p>
        )}
        <ul className="palette__list" role="listbox" aria-label="Commands">
          {matches.map(({ command, arg }, i) => (
            <li key={command.id} role="option" aria-selected={i === index}>
              <button
                type="button"
                className={`palette__item${i === index ? " palette__item--on" : ""}`}
                onMouseEnter={() => setIndex(i)}
                onClick={() => void run(i)}
              >
                <span className="palette__title">
                  {command.title}
                  {arg && <span className="palette__arg"> · {arg}</span>}
                </span>
                {command.hint && !arg && <span className="palette__hint">{command.hint}</span>}
                {i === index && <CornerDownLeft size={12} aria-hidden className="palette__enter" />}
              </button>
            </li>
          ))}
          {matches.length === 0 && <li className="palette__empty">No command matches</li>}
        </ul>
      </div>
    </ModuleContainer>
  );
}
