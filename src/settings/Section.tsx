import type { ReactNode } from "react";

/** A titled settings page. */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  const id = `section-${title.toLowerCase().replace(/[^a-z]+/g, "-")}`;
  return (
    <section className="settings__section" aria-labelledby={id}>
      <h1 id={id} className="settings__title">
        {title}
      </h1>
      {children}
    </section>
  );
}
