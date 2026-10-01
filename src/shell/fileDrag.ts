import { create } from "zustand";

/** Set while files are dragged *out* of the notch, so passing over it isn't a drop. */
export const useFileDrag = create<{ draggingOut: boolean }>(() => ({ draggingOut: false }));
