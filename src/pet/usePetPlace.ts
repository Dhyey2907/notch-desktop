// There is one pet: in the notch, or out walking the taskbar. This keeps track of which, so the
// notch only draws it when it's actually home.
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { ipc } from "../core/ipc";
import { useSettings } from "../settings";
import { petOut, usePetLive } from "./petStore";

/** Rust → every window: the pet started or finished a walk. */
export const PET_WALKING_EVENT = "pet-walking";

/** Call once per window: follows the pet's walks. */
export function usePetWalkLink() {
  useEffect(() => {
    let live = true;
    void ipc
      .petWalking()
      .then((walking) => live && usePetLive.setState({ walking }))
      .catch(() => {});
    const off = listen<boolean>(PET_WALKING_EVENT, ({ payload }) =>
      usePetLive.setState({ walking: payload }),
    );
    return () => {
      live = false;
      void off.then((f) => f());
    };
  }, []);
}

/** The pet is out of the notch (walking the taskbar, or just called out). */
export function usePetOut(): boolean {
  const calledOut = useSettings((s) => s.settings?.pet.buddyOut ?? false);
  const walking = usePetLive((s) => s.walking);
  return petOut(calledOut, walking);
}
