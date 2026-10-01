// Vault: logins kept as notes in your Obsidian vault (passwords encrypted; the key stays in
// Windows Credential Manager) and 2FA codes. Copy a username, password or code in a click;
// passwords and codes stay out of clipboard history and are cleared after 30 seconds.
import { KeyRound } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { useVault } from "./vaultStore";

function init({ events }: ModuleContext) {
  const off = events.on("VaultChanged", ({ event }) => useVault.getState().set(event.payload));
  ipc.vaultState().then(
    (state) => useVault.getState().set(state),
    () => {},
  );
  return off;
}

export const vaultModule: NotchModule = {
  id: "vault",
  name: "Vault",
  icon: KeyRound,
  order: 57,
  init,
  Expanded: lazy(() => import("./VaultExpanded")),
};
