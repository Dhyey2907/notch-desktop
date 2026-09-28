import { Button, ConfirmButton, Dot, Pill } from "../design/components";
import { findModule } from "../modules/registry";
import type { Activity } from "./activityStore";
import "./activity.css";

function relativeTime(from: number, now: number): string {
  const minutes = Math.floor((now - from) / 60_000);
  if (minutes < 1) return "now";
  if (minutes < 60) return `${minutes}m`;
  return `${Math.floor(minutes / 60)}h`;
}

/**
 * One live activity in the island (design.md §10, §16). Critical activities get a distinct
 * eyebrow with a warning accent; everything else reads like a quiet notification.
 */
export function ActivityView({
  activity,
  queued,
  onRunAction,
  onOpen,
}: {
  activity: Activity;
  /** How many other activities are waiting behind this one. */
  queued: number;
  onRunAction: (actionId: string) => void;
  onOpen: () => void;
}) {
  const moduleName = findModule(activity.moduleId)?.name ?? activity.moduleId;
  const critical = activity.priority === "critical";
  const tone = activity.tone ?? (critical ? "warning" : "neutral");

  return (
    <article
      className={`activity activity--${activity.priority}`}
      aria-live={critical ? "assertive" : "polite"}
      aria-label={`${moduleName}: ${activity.title}`}
    >
      <button type="button" className="activity__open" onClick={onOpen} aria-label={`Open ${moduleName}`} />
      <div className="activity__main">
        {activity.imageUrl ? (
          <img className="activity__image" src={activity.imageUrl} alt="" />
        ) : activity.avatar ? (
          <span className="nd-avatar" aria-hidden>
            {activity.avatar}
          </span>
        ) : (
          <Dot tone={tone} />
        )}
        <div className="activity__text">
          {critical && <p className="nd-eyebrow activity__eyebrow">{moduleName} needs you</p>}
          <p className="activity__title">
            {activity.title}
            {activity.repeats > 1 && <Pill>×{activity.repeats}</Pill>}
          </p>
          {activity.body && <p className="activity__body">{activity.body}</p>}
        </div>
        <div className="activity__meta">
          <time>{relativeTime(activity.createdAt, activity.updatedAt)}</time>
          {queued > 0 && <Pill>+{queued} more</Pill>}
        </div>
      </div>
      {activity.actions && activity.actions.length > 0 && (
        <div className="activity__actions">
          {activity.actions.map((action) =>
            action.confirmLabel ? (
              <ConfirmButton
                key={action.id}
                variant={action.variant === "danger" ? "danger" : "warning"}
                confirmLabel={action.confirmLabel}
                onConfirm={() => onRunAction(action.id)}
              >
                {action.label}
              </ConfirmButton>
            ) : (
              <Button key={action.id} variant={action.variant} onClick={() => onRunAction(action.id)}>
                {action.label}
              </Button>
            ),
          )}
        </div>
      )}
    </article>
  );
}
