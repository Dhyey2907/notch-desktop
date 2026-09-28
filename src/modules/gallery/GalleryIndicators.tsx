import { Dot, MockBadge } from "../../design/components";

export function GalleryCompact() {
  return <Dot tone="info" label="Design gallery (development)" />;
}

export function GalleryPeek() {
  return (
    <div className="module-peek">
      <span className="module-peek__title">
        Design gallery <MockBadge />
      </span>
      <span className="module-peek__meta">Sample content for visual review</span>
    </div>
  );
}
