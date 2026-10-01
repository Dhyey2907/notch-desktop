import { ChevronDown, ChevronUp, GripVertical } from "lucide-react";
import { useState } from "react";

import { IconButton, MockBadge, Pill, Surface, Switch } from "../design/components";
import { MODULES } from "../modules/registry";
import { dockOrder } from "../shell/useModules";
import { useSettings } from ".";
import { Row, Select } from "./controls";
import { patch, resetGroups } from "./patch";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";
import { moduleSettings } from "./settingsStore";

const LAST_USED = "last";

export function DockSection() {
  const settings = useSettings((s) => s.settings);
  const setModule = useSettings((s) => s.setModule);
  const [dragging, setDragging] = useState<string | null>(null);
  if (!settings) return null;
  const { dock } = settings;
  const ordered = dockOrder(MODULES, dock.order);
  const ids = ordered.map((m) => m.id);
  const enabled = ordered.filter((m) => m.alwaysOn || moduleSettings(settings, m.id).enabled);

  const move = (id: string, to: number) => {
    const next = ids.filter((x) => x !== id);
    next.splice(Math.max(0, Math.min(next.length, to)), 0, id);
    patch("dock", { order: next });
  };
  const setDocked = (id: string, shown: boolean) =>
    patch("dock", { hidden: shown ? dock.hidden.filter((h) => h !== id) : [...dock.hidden, id] });

  return (
    <Section title="Dock & modules" lead={pageLead("dock")} onReset={() => void resetGroups(["dock"])}>
      <Surface>
        <Row label="Open the notch to" description="Which module shows when you open the notch.">
          <Select
            label="Open the notch to"
            value={dock.first ?? LAST_USED}
            options={[
              { value: LAST_USED, label: "Wherever I left it" },
              ...enabled.map((m) => ({ value: m.id, label: m.name })),
            ]}
            onChange={(v) => patch("dock", { first: v === LAST_USED ? undefined : v })}
          />
        </Row>
      </Surface>
      <p id="setting-modules" className="settings__lead">
        Drag to reorder the dock (or use the arrows). <strong>On</strong> runs the module;{" "}
        <strong>In the dock</strong> shows its icon; <strong>Can interrupt</strong> lets it show
        notifications.
      </p>
      <ol className="dock-list">
        {ordered.map((module, index) => {
          const prefs = moduleSettings(settings, module.id);
          const on = module.alwaysOn || prefs.enabled;
          const Icon = module.icon;
          return (
            <li
              key={module.id}
              className={`dock-list__item${dragging === module.id ? " dock-list__item--dragging" : ""}`}
              draggable
              onDragStart={(e) => {
                setDragging(module.id);
                e.dataTransfer.effectAllowed = "move";
              }}
              onDragEnd={() => setDragging(null)}
              onDragOver={(e) => {
                if (!dragging || dragging === module.id) return;
                e.preventDefault();
              }}
              onDrop={(e) => {
                e.preventDefault();
                if (dragging && dragging !== module.id) move(dragging, index);
                setDragging(null);
              }}
            >
              <Surface>
                <div className="settings__module-head">
                  <GripVertical size={14} className="dock-list__grip" aria-hidden />
                  <Icon aria-hidden size={16} />
                  <span className="settings__row-label">{module.name}</span>
                  {module.devOnly && <MockBadge />}
                  {module.alwaysOn && <Pill>Always on</Pill>}
                  <span className="dock-list__spacer" />
                  <IconButton
                    icon={ChevronUp}
                    label={`Move ${module.name} up`}
                    size="sm"
                    disabled={index === 0}
                    onClick={() => move(module.id, index - 1)}
                  />
                  <IconButton
                    icon={ChevronDown}
                    label={`Move ${module.name} down`}
                    size="sm"
                    disabled={index === ordered.length - 1}
                    onClick={() => move(module.id, index + 1)}
                  />
                </div>
                {prefs.autoDisabled && !prefs.enabled && (
                  <p className="settings__note settings__note--warning" role="alert">
                    Turned off after failing to start three times in a row. Turn it back on to try again.
                  </p>
                )}
                <div className="dock-list__switches">
                  <Switch
                    label="On"
                    checked={on}
                    disabled={module.alwaysOn}
                    onChange={(enabled) =>
                      void setModule(module.id, { enabled, autoDisabled: false, initFailures: 0 })
                    }
                  />
                  <Switch
                    label="In the dock"
                    checked={on && !dock.hidden.includes(module.id)}
                    disabled={!on}
                    onChange={(shown) => setDocked(module.id, shown)}
                  />
                  <Switch
                    label="Can interrupt"
                    checked={prefs.mayPeek}
                    disabled={!on}
                    onChange={(mayPeek) => void setModule(module.id, { mayPeek })}
                  />
                </div>
              </Surface>
            </li>
          );
        })}
      </ol>
      <p className="settings__note">
        Approvals from Claude Code always appear, even if Claude can't interrupt.
      </p>
    </Section>
  );
}
