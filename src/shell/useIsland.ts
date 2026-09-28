import { ipc } from "../core/ipc";
import { createIslandStore } from "./islandStore";

/** The app-wide island store, wired to the backend window-resize command. */
export const useIsland = createIslandStore((size) => ipc.setWindowSize(size.width, size.height));
