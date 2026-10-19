// Edit layout: shown in place of the open panel. Drag (or use the arrows) to reorder a panel's
// sections, the eye hides one, and the panel's size and everyone's spacing are chosen here.
import { ArrowDown, ArrowUp, Eye, EyeOff, GripVertical } from "lucide-react";
import { useState, type DragEvent } from "react";

import type { Density } from "../core/generated/Density";
import type { PanelSize } from "../core/generated/PanelSize";
import { SegmentedControl } from "../design/components";
import type { NotchModule } from "../modules/types";
import { useSettings } from "../settings";
import {
  nudge,
  ordered,
  PANEL_SIZE_LABELS,
  resetPanel,
  savePanel,
  useLayoutEdit,
  usePanelLayout,
  type LayoutGroup,
} from "./layout";
import "./layoutEditor.css";

const SIZES = Object.keys(PANEL_SIZE_LABELS) as PanelSize[];
const DENSITIES: { value: Density; label: string }[] = [
  { value: "compact", label: "Compact" },
  { value: "comfortable", label: "Comfortable" },
  { value: "spacious", label: "Spacious" },
];

function GroupEditor({ group }: { group: LayoutGroup }) {
  const layout = usePanelLayout(group.id);
  const [dragging, setDragging] = useState<string | null>(null);
  const items = ordered(group.items, layout);
  const ids = items.map((i) => i.id);
  const hidden = layout?.hidden ?? [];
  const shown = ids.filter((id) => !hidden.includes(id));

  const move = (next: string[]) => void savePanel(group.id, { order: next });
  const toggle = (id: string) => {
    const isHidden = hidden.includes(id);
    if (!isHidden && group.keepOne && shown.length <= 1) return;
    void savePanel(group.id, { hidden: isHidden ? hidden.filter((h) => h !== id) : [...hidden, id] });
  };
  const over = (e: DragEvent, id: string) => {
    if (!dragging || dragging === id) return;
    e.preventDefault();
    const rest = ids.filter((x) => x !== dragging);
    const at = rest.indexOf(id) + (ids.indexOf(dragging) < ids.indexOf(id) ? 1 : 0);
    move([...rest.slice(0, at), dragging, ...rest.slice(at)]);
  };

  return (
    <div className="layout-edit__group">
      <p className="layout-edit__label">{group.label}</p>
      <ul className="layout-edit__list" aria-label={group.label}>
        {items.map((item, i) => {
          const off = hidden.includes(item.id);
          return (
            <li
              key={item.id}
              className={`layout-edit__item${off ? " layout-edit__item--off" : ""}${dragging === item.id ? " layout-edit__item--drag" : ""}`}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.effectAllowed = "move";
                setDragging(item.id);
              }}
              onDragOver={(e) => over(e, item.id)}
              onDragEnd={() => setDragging(null)}
            >
              <GripVertical size={13} aria-hidden className="layout-edit__grip" />
              <span className="layout-edit__name">{item.label}</span>
              <button
                type="button"
                className="layout-edit__btn"
                aria-label={`Move ${item.label} up`}
                disabled={i === 0}
                onClick={() => move(nudge(ids, item.id, -1))}
              >
                <ArrowUp size={12} aria-hidden />
              </button>
              <button
                type="button"
                className="layout-edit__btn"
                aria-label={`Move ${item.label} down`}
                disabled={i === items.length - 1}
                onClick={() => move(nudge(ids, item.id, 1))}
              >
                <ArrowDown size={12} aria-hidden />
              </button>
              <button
                type="button"
                className="layout-edit__btn"
                aria-label={off ? `Show ${item.label}` : `Hide ${item.label}`}
                aria-pressed={!off}
                disabled={!off && group.keepOne && shown.length <= 1}
                onClick={() => toggle(item.id)}
              >
                {off ? <EyeOff size={12} aria-hidden /> : <Eye size={12} aria-hidden />}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

export function LayoutEditor({ module }: { module: NotchModule }) {
  const layout = usePanelLayout(module.id);
  const density = useSettings((s) => s.settings?.layout?.density ?? "comfortable");
  const groups = module.layout ?? [];
  const setDensity = (d: Density) =>
    void useSettings.getState().update((s) => ({ ...s, layout: { ...s.layout, density: d } }));

  return (
    <section className="layout-edit" aria-label={`${module.name} layout`}>
      <header className="layout-edit__head">
        <h2 className="layout-edit__title">{module.name} layout</h2>
        <button type="button" className="nd-chip" onClick={() => void resetPanel(module.id)}>
          Reset
        </button>
        <button
          type="button"
          className="nd-chip layout-edit__done"
          onClick={() => useLayoutEdit.getState().set(false)}
        >
          Done
        </button>
      </header>
      <div className="layout-edit__body">
        <div className="layout-edit__row">
          <p className="layout-edit__label">Panel size</p>
          <SegmentedControl
            label="Panel size"
            value={layout?.size ?? "normal"}
            options={SIZES.map((s) => ({ value: s, label: PANEL_SIZE_LABELS[s] }))}
            onChange={(size) => void savePanel(module.id, { size })}
          />
        </div>
        <div className="layout-edit__row">
          <p className="layout-edit__label">Spacing · every panel</p>
          <SegmentedControl label="Spacing" value={density} options={DENSITIES} onChange={setDensity} />
        </div>
        {groups.length === 0 ? (
          <p className="layout-edit__note">
            {module.name} is a single view: its size and spacing can change, but there are no sections to
            move.
          </p>
        ) : (
          groups.map((g) => <GroupEditor key={g.id} group={g} />)
        )}
      </div>
    </section>
  );
}
