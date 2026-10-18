// Settings → Dock & modules: dock profiles (a different dock for work, games or study, picked by
// the app in front or the time of day) and the widget pinned beside the resting bubble.
import { Plus, Trash2 } from "lucide-react";

import type { DockProfile } from "../core/generated/DockProfile";
import type { DockSettings } from "../core/generated/DockSettings";
import { Button, Surface, Switch, TextField } from "../design/components";
import { starterProfiles } from "../shell/dockProfiles";
import { MINI_WIDGETS } from "../shell/miniWidgets";
import { useSettings } from ".";
import { Row, Select } from "./controls";

const ANY = "any";
const HOURS = Array.from({ length: 24 }, (_, h) => ({
  value: String(h * 60),
  label: new Date(2000, 0, 1, h).toLocaleTimeString(undefined, { hour: "numeric" }),
}));

export function DockProfilesPanel() {
  const dock = useSettings((s) => s.settings?.dock);
  if (!dock) return null;
  const update = (change: (d: DockSettings) => DockSettings) =>
    void useSettings.getState().update((s) => ({ ...s, dock: change(s.dock) }));
  const active = dock.profiles.find((p) => p.id === dock.active) ?? null;
  const setProfile = (id: string, patch: Partial<DockProfile>) =>
    update((d) => ({ ...d, profiles: d.profiles.map((p) => (p.id === id ? { ...p, ...patch } : p)) }));

  return (
    <Surface>
      <div id="setting-dock-profiles">
        <Row
          label="Dock profiles"
          description="A different dock for each kind of work. Rearrange the one in use here or by long-pressing a dock icon in the notch."
        >
          <Select
            label="Dock in use"
            value={dock.active}
            options={[
              { value: "", label: "Main dock" },
              ...dock.profiles.map((p) => ({ value: p.id, label: p.name })),
            ]}
            onChange={(v) => update((d) => ({ ...d, active: v, auto: false }))}
          />
        </Row>
        {dock.profiles.length === 0 ? (
          <Button onClick={() => update((d) => ({ ...d, profiles: starterProfiles(d.order, d.hidden) }))}>
            <Plus size={13} aria-hidden /> Add Work, Gaming and Study profiles
          </Button>
        ) : (
          <Switch
            label="Switch by themselves"
            description="To a profile listing the app in front, else one whose hours it is, else the main dock."
            checked={dock.auto}
            onChange={(auto) => update((d) => ({ ...d, auto }))}
          />
        )}
        {active && (
          <div className="dock-profile">
            <TextField
              label="Name"
              value={active.name}
              onChange={(name) => setProfile(active.id, { name: name.slice(0, 30) })}
            />
            <TextField
              label="Apps that switch to it"
              description="Program names, separated by commas — e.g. steam, valorant, code."
              value={active.apps.join(", ")}
              onChange={(text) =>
                setProfile(active.id, {
                  apps: text
                    .split(",")
                    .map((a) => a.trim().toLowerCase())
                    .filter(Boolean)
                    .slice(0, 20),
                })
              }
            />
            <Row label="Hours" description="Used when no listed app is in front.">
              <Select
                label="From"
                value={active.fromMin === undefined ? ANY : String(active.fromMin)}
                options={[{ value: ANY, label: "Any time" }, ...HOURS]}
                onChange={(v) =>
                  setProfile(active.id, {
                    fromMin: v === ANY ? undefined : Number(v),
                    toMin: v === ANY ? undefined : (active.toMin ?? (Number(v) + 8 * 60) % (24 * 60)),
                  })
                }
              />
              {active.fromMin !== undefined && (
                <Select
                  label="Until"
                  value={String(active.toMin ?? 0)}
                  options={HOURS}
                  onChange={(v) => setProfile(active.id, { toMin: Number(v) })}
                />
              )}
            </Row>
            <Button
              onClick={() =>
                update((d) => ({ ...d, active: "", profiles: d.profiles.filter((p) => p.id !== active.id) }))
              }
            >
              <Trash2 size={13} aria-hidden /> Delete this profile
            </Button>
          </div>
        )}
      </div>
      <div id="setting-widget-beside-the-bubble">
        <Row
          label="Widget beside the bubble"
          description="Always shown next to the resting bubble when it has something to say."
        >
          <Select
            label="Widget beside the bubble"
            value={dock.pinned ?? ""}
            options={[
              { value: "", label: "None" },
              ...MINI_WIDGETS.map((w) => ({ value: w.id, label: w.label })),
            ]}
            onChange={(v) => update((d) => ({ ...d, pinned: v || undefined }))}
          />
        </Row>
      </div>
    </Surface>
  );
}
