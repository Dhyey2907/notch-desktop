import { ipc } from "../core/ipc";
import { uiScale } from "./appearance";
import { createIslandStore } from "./islandStore";

/** The app-wide island store, wired to the backend window-resize command. */
export const useIsland = createIslandStore((size) =>
  // The page is zoomed by the notch size setting; the window must grow with it.
  ipc.setWindowSize(Math.ceil(size.width * uiScale()), Math.ceil(size.height * uiScale())),
);
