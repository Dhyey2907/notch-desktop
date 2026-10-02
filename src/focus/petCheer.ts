// When the pet cheers. Kept outside the pet itself: the pet only shows in the collapsed bubble,
// and the things it cheers for (a finished focus session, a ticked-off task, a break taken)
// usually happen while the notch is open.
import { create } from "zustand";

export const CHEER_MS = 6000;

export const usePetCheer = create<{ until: number }>(() => ({ until: 0 }));

export function cheerPet(now = Date.now()) {
  usePetCheer.setState({ until: now + CHEER_MS });
}
