import { Button, ConfirmButton, Dot, IconButton, Pill } from "../design/components";
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
  const actions = activity.actions ?? [];
  // Icon-only actions (media controls) sit inline on the main row; text actions get their own row.
  const inline = actions.length > 0 && actions.every((a) => a.icon);
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
        {inline ? (
          <div className="activity__inline-actions" role="group" aria-label={`${moduleName} controls`}>
            {actions.map(
              (action) =>
                action.icon && (
                  <IconButton
                    key={action.id}
                    icon={action.icon}
                    label={action.label}
                    disabled={action.disabled}
                    onClick={() => onRunAction(action.id)}
                  />
                ),
            )}
          </div>
        ) : (
          <div className="activity__meta">
            <time>{relativeTime(activity.createdAt, activity.updatedAt)}</time>
            {queued > 0 && <Pill>+{queued} more</Pill>}
          </div>
        )}
      </div>
      {activity.code && (
        <pre className="activity__code" tabIndex={0} aria-label="Exact command">
          {activity.code}
        </pre>
      )}
      {!inline && actions.length > 0 && (
        <div className="activity__actions">
          {actions.map((action) =>
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
              <Button
                key={action.id}
                variant={action.variant}
                disabled={action.disabled}
                onClick={() => onRunAction(action.id)}
              >
                {action.label}
              </Button>
            ),
          )}
        </div>
      )}
    </article>
  );
}
