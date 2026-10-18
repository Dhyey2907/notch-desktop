// A sticky note on the desktop (#sticky=<note id>): drag the top bar to move it, the corner to
// resize, pick a colour, type to edit (saved to the note). × takes it off the desktop; the note
// stays in Notes.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { TodoSnapshot } from "../core/generated/TodoSnapshot";
import type { Sticky } from "../core/generated/Sticky";
import "./sticky.css";

const COLORS = ["yellow", "pink", "blue", "green", "gray"] as const;
const SAVE_MS = 600;

export function StickyApp() {
  const noteId = decodeURIComponent(window.location.hash.replace(/^#sticky=/, ""));
  const [text, setText] = useState<string | null>(null);
  const [color, setColor] = useState<string>("yellow");
  const saveTimer = useRef<number | undefined>(undefined);
  const placeTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    invoke<TodoSnapshot>("todo_snapshot").then(
      (snap) => setText(snap.notes.find((n) => n.id === noteId)?.body ?? ""),
      () => setText(""),
    );
    invoke<Sticky[]>("sticky_list").then(
      (list) => setColor(list.find((s) => s.noteId === noteId)?.color ?? "yellow"),
      () => {},
    );
  }, [noteId]);

  // Remember where it was moved or resized to.
  useEffect(() => {
    const w = getCurrentWindow();
    const remember = () => {
      window.clearTimeout(placeTimer.current);
      placeTimer.current = window.setTimeout(async () => {
        const [pos, size] = await Promise.all([w.outerPosition(), w.outerSize()]);
        void invoke("sticky_place", { noteId, x: pos.x, y: pos.y, w: size.width, h: size.height }).catch(
          () => {},
        );
      }, 500);
    };
    const offs = [w.onMoved(remember), w.onResized(remember)];
    return () => {
      window.clearTimeout(placeTimer.current);
      offs.forEach((p) => void p.then((off) => off()));
    };
  }, [noteId]);

  const edit = (body: string) => {
    setText(body);
    window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      void invoke("note_update", { id: noteId, body }).catch(() => {});
    }, SAVE_MS);
  };

  return (
    <div className={`sticky sticky--${color}`}>
      <div className="sticky__bar" data-tauri-drag-region>
        <span className="sticky__colors" role="radiogroup" aria-label="Colour">
          {COLORS.map((c) => (
            <button
              key={c}
              type="button"
              role="radio"
              aria-checked={c === color}
              aria-label={c}
              className={`sticky__dot sticky__dot--${c}`}
              onClick={() => {
                setColor(c);
                void invoke("sticky_color", { noteId, color: c }).catch(() => {});
              }}
            />
          ))}
        </span>
        <button
          type="button"
          className="sticky__close"
          aria-label="Take off the desktop (the note stays in Notes)"
          title="Take off the desktop (the note stays in Notes)"
          onClick={() => void invoke("sticky_unpin", { noteId })}
        >
          <X size={13} aria-hidden />
        </button>
      </div>
      {text !== null && (
        <textarea
          className="sticky__text"
          aria-label="Note"
          value={text}
          maxLength={10_000}
          onChange={(e) => edit(e.target.value)}
        />
      )}
    </div>
  );
}
