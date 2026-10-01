import { create } from "zustand";

import type { GithubNotification } from "../../core/generated/GithubNotification";
import type { GithubStatus } from "../../core/generated/GithubStatus";

/** GitHub sign-in state and unread notifications, kept current by the Git module's init. */
export const useGithub = create<{
  status: GithubStatus | null;
  set: (s: GithubStatus) => void;
  notifications: GithubNotification[];
  setNotifications: (list: GithubNotification[]) => void;
}>((set) => ({
  status: null,
  set: (status) => set({ status }),
  notifications: [],
  setNotifications: (notifications) => set({ notifications }),
}));
