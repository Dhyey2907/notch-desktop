import { create } from "zustand";

import type { GithubStatus } from "../../core/generated/GithubStatus";

/** GitHub sign-in state, kept current by the Git module's init. */
export const useGithub = create<{ status: GithubStatus | null; set: (s: GithubStatus) => void }>((set) => ({
  status: null,
  set: (status) => set({ status }),
}));
