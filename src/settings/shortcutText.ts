// Shortcut text as the backend parses it ("Ctrl+Alt+N"), from key presses and for display.

/** `KeyN` → `N`, `Digit1` → `1`; other codes (Space, F5, ArrowUp) stay as they are. */
export function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^(F\d{1,2}|Space|Arrow(Up|Down|Left|Right)|Home|End|PageUp|PageDown|Insert|Delete)$/.test(code))
    return code;
  if (
    /^(Comma|Period|Slash|Backslash|Semicolon|Quote|BracketLeft|BracketRight|Minus|Equal|Backquote)$/.test(
      code,
    )
  )
    return code;
  return null;
}

/** "Ctrl+Alt+N" from a key press, or null while only modifiers are held. */
export function shortcutFrom(e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "code">) {
  const key = keyName(e.code);
  if (!key) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(
    Boolean,
  ) as string[];
  // A bare letter would fire while typing anywhere; function keys are fine alone.
  if (mods.length === 0 && !/^F\d/.test(key)) return null;
  if (mods.length === 1 && mods[0] === "Shift" && !/^F\d/.test(key)) return null;
  return [...mods, key].join("+");
}

/** "Ctrl+Alt+N" → "Ctrl + Alt + N" (Super shows as Win). */
export function prettyShortcut(text: string): string {
  return text
    .split("+")
    .map((p) => (p === "Super" ? "Win" : p.replace(/^Arrow/, "")))
    .join(" + ");
}
