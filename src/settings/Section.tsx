import { RotateCcw } from "lucide-react";
import type { ReactNode } from "react";

import { ConfirmButton } from "../design/components";

/** A titled settings page, with a one-line explanation and an optional "Reset this page". */
export function Section({
  title,
  lead,
  onReset,
  children,
}: {
  title: string;
  lead?: string;
  onReset?: () => void;
  children: ReactNode;
}) {
  const id = `section-${title.toLowerCase().replace(/[^a-z]+/g, "-")}`;
  return (
    <section className="settings__section" aria-labelledby={id}>
      <header className="settings__head">
        <div>
          <h1 id={id} className="settings__title">
            {title}
          </h1>
          {lead && <p className="settings__page-lead">{lead}</p>}
        </div>
        {onReset && (
          <ConfirmButton confirmLabel="Reset to defaults?" onConfirm={onReset}>
            <RotateCcw size={13} aria-hidden /> Reset this page
          </ConfirmButton>
        )}
      </header>
      {children}
    </section>
  );
}
