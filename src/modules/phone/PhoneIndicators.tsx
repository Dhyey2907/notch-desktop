import { usePhone } from "./phoneStore";

/** Hover: the latest phone notification. */
export function PhonePeek() {
  const snapshot = usePhone((s) => s.snapshot);
  const latest = snapshot?.notifications[0];
  return (
    <div className="module-peek">
      <span className="module-peek__title">{latest ? latest.title : "Phone"}</span>
      <span className="module-peek__meta">
        {latest
          ? latest.body || "Phone Link"
          : snapshot?.access === "allowed"
            ? "No phone notifications"
            : "Click to set up"}
      </span>
    </div>
  );
}
