import { describe, expect, it, vi } from "vitest";

import type { ApprovalRequest } from "../../core/generated/ApprovalRequest";

vi.mock("../../core/ipc", () => ({ ipc: { claudeApprovalDecide: vi.fn(() => Promise.resolve()) } }));

import { ipc } from "../../core/ipc";
import { approvalActivity } from "./approvals";

const request = (overrides: Partial<ApprovalRequest> = {}): ApprovalRequest => ({
  id: "r1",
  agent: "claude",
  sessionId: "s1",
  project: "notch-desktop",
  tool: "Bash",
  summary: "pnpm test",
  destructive: false,
  reasons: [],
  createdAt: "2026-09-29T10:00:00Z",
  expiresAt: "2026-09-29T10:01:50Z",
  ...overrides,
});

describe("approvalActivity", () => {
  it("is a persistent critical card showing the exact command", () => {
    const activity = approvalActivity(request());
    expect(activity.priority).toBe("critical");
    expect(activity.persistent).toBe(true);
    expect(activity.code).toBe("pnpm test");
    expect(activity.actions?.map((a) => a.id)).toEqual(["terminal", "deny", "session", "allow"]);
    expect(activity.actions?.find((a) => a.id === "allow")?.confirmLabel).toBeUndefined();
  });

  it("asks twice for destructive commands and never offers a session rule", () => {
    const activity = approvalActivity(
      request({ summary: "rm -rf build", destructive: true, reasons: ["Recursively deletes files"] }),
    );
    expect(activity.tone).toBe("danger");
    expect(activity.body).toContain("Recursively deletes files");
    expect(activity.actions?.map((a) => a.id)).toEqual(["terminal", "deny", "allow"]);
    expect(activity.actions?.find((a) => a.id === "allow")?.confirmLabel).toBeTruthy();
  });

  it("sends the chosen answer for this request", async () => {
    await approvalActivity(request())
      .actions?.find((a) => a.id === "deny")
      ?.run();
    expect(ipc.claudeApprovalDecide).toHaveBeenCalledWith("r1", "deny");
  });
});

describe("other agents", () => {
  it("names the agent that's asking", () => {
    expect(approvalActivity(request({ agent: "codex" })).title).toBe("Codex wants to run a command");
    expect(approvalActivity(request({ agent: "codex", tool: "apply_patch" })).title).toBe(
      "Codex wants to edit files",
    );
  });
});
