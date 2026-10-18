// When to show the clipboard preview: after the pointer rests on an entry for a moment, at
// once when moving between entries while one is showing, and gone as soon as it leaves.
export const HOVER_DELAY_MS = 350;

export interface HoverPreview {
  enter: (id: string) => void;
  leave: () => void;
  dispose: () => void;
}

export function createHoverPreview(show: (id: string | null) => void, delay = HOVER_DELAY_MS): HoverPreview {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let shown: string | null = null;
  const clear = () => {
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
  };
  return {
    enter(id) {
      clear();
      if (shown !== null) {
        shown = id;
        show(id);
        return;
      }
      timer = setTimeout(() => {
        timer = undefined;
        shown = id;
        show(id);
      }, delay);
    },
    leave() {
      clear();
      if (shown !== null) {
        shown = null;
        show(null);
      }
    },
    dispose: clear,
  };
}
