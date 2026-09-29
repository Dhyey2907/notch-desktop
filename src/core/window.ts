// Which island this webview is. With "Show the notch on: All screens" every screen runs its
// own copy of the app; `main` is the island on the lead (primary) screen.
import { getCurrentWindow } from "@tauri-apps/api/window";

let cached: string | null = null;

export function windowLabel(): string {
  if (cached === null) {
    try {
      cached = getCurrentWindow().label;
    } catch {
      cached = "main"; // tests and non-Tauri previews
    }
  }
  return cached;
}

/** One-per-app duties (first-run prompt, timer rings) happen only in the lead island. */
export function isLeadIsland(): boolean {
  return windowLabel() === "main";
}
