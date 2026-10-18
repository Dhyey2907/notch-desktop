// Shown in the dock while you rearrange it: which dock profile you're editing, and which small
// widget is pinned beside the resting bubble.
import { useSettings } from "../settings";
import { MINI_WIDGETS } from "./miniWidgets";

export function DockEditBar() {
  const dock = useSettings((s) => s.settings?.dock);
  if (!dock) return null;
  const set = (patch: Partial<typeof dock>) =>
    void useSettings.getState().update((s) => ({ ...s, dock: { ...s.dock, ...patch } }));
  return (
    <span className="dock-edit">
      <select
        className="dock-edit__select"
        aria-label="Dock profile"
        value={dock.active}
        onChange={(e) => set({ active: e.target.value, auto: false })}
      >
        <option value="">Main dock</option>
        {dock.profiles.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
      <select
        className="dock-edit__select"
        aria-label="Widget beside the bubble"
        value={dock.pinned ?? ""}
        onChange={(e) => set({ pinned: e.target.value || undefined })}
      >
        <option value="">No widget</option>
        {MINI_WIDGETS.map((w) => (
          <option key={w.id} value={w.id}>
            {w.label}
          </option>
        ))}
      </select>
    </span>
  );
}
