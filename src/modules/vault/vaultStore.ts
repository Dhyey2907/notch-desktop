import { create } from "zustand";

import type { VaultEntry } from "../../core/generated/VaultEntry";
import type { VaultState } from "../../core/generated/VaultState";

export type VaultTab = "logins" | "codes";

interface VaultStore {
  state: VaultState | null;
  set: (state: VaultState) => void;
  tab: VaultTab;
  setTab: (tab: VaultTab) => void;
}

export const useVault = create<VaultStore>((set) => ({
  state: null,
  set: (state) => set({ state }),
  tab: "logins",
  setTab: (tab) => set({ tab }),
}));

/** Search titles, usernames, sites and tags. */
export function matches(entry: VaultEntry, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return [entry.title, entry.username, entry.url, ...entry.tags].some((field) =>
    field.toLowerCase().includes(q),
  );
}

/** "https://github.com/login" → "github.com". */
export function siteOf(url: string): string {
  if (!url) return "";
  try {
    return new URL(url.includes("://") ? url : `https://${url}`).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
