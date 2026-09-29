// Agent approvals (plan.md Phase 10): each Claude Code permission request waiting on you
// becomes a critical, persistent activity showing the exact command. Answers go back through
// the hook; anything you don't answer here falls back to Claude Code's own terminal prompt.
import type { ActivityAction, ActivityInput } from "../../activity/activityStore";
import type { ApprovalChoice } from "../../core/generated/ApprovalChoice";
import type { ApprovalRequest } from "../../core/generated/ApprovalRequest";
import { ipc } from "../../core/ipc";

const decide = (id: string, choice: ApprovalChoice) => () => ipc.claudeApprovalDecide(id, choice);

/** What the tool wants to do, in plain words. */
export function approvalTitle(request: ApprovalRequest): string {
  switch (request.tool) {
    case "Bash":
    case "PowerShell":
      return "Claude Code wants to run a command";
    case "Write":
    case "Edit":
    case "MultiEdit":
    case "NotebookEdit":
      return "Claude Code wants to edit a file";
    case "WebFetch":
      return "Claude Code wants to open a web page";
    default:
      return `Claude Code wants to use ${request.tool}`;
  }
}

export function approvalActivity(request: ApprovalRequest): ActivityInput {
  const where = request.project ? ` · ${request.project}` : "";
  const allow: ActivityAction = request.destructive
    ? {
        id: "allow",
        label: "Allow",
        variant: "danger",
        confirmLabel: "Really allow?",
        run: decide(request.id, "allow"),
      }
    : { id: "allow", label: "Allow", variant: "primary", run: decide(request.id, "allow") };
  return {
    moduleId: "claude",
    priority: "critical",
    persistent: true,
    tone: request.destructive ? "danger" : "warning",
    dedupKey: `claude-approval-${request.id}`,
    countRepeats: false,
    title: approvalTitle(request),
    body: request.destructive ? `${request.reasons.join(" · ")}${where}` : `${request.tool}${where}`,
    code: request.summary,
    actions: [
      {
        id: "terminal",
        label: "In terminal",
        variant: "secondary",
        run: decide(request.id, "answerInTerminal"),
      },
      { id: "deny", label: "Deny", variant: "secondary", run: decide(request.id, "deny") },
      // Never offered for destructive requests: each one should get its own look.
      ...(request.destructive
        ? []
        : [
            {
              id: "session",
              label: "Allow for session",
              variant: "secondary" as const,
              run: decide(request.id, "allowForSession"),
            },
          ]),
      allow,
    ],
  };
}
