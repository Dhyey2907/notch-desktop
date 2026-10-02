import type { RoutineStep } from "../core/generated/RoutineStep";

/** "Open chrome.exe", "Open github.com", "Close Discord.exe", "Focus 50 min". */
export function stepLabel(step: RoutineStep): string {
  const name = (path: string) => path.split(/[\\/]/).pop() ?? path;
  switch (step.kind) {
    case "app":
      return `Open ${name(step.path).replace(/\.(lnk|exe)$/i, "")}`;
    case "url":
      return `Open ${step.url.replace(/^https?:\/\/(www\.)?/, "").replace(/\/$/, "")}`;
    case "folder":
      return `Open folder ${name(step.path)}`;
    case "close":
      return `Close ${step.exe.replace(/\.exe$/i, "")}`;
    case "focus":
      return `Focus for ${step.minutes} min`;
  }
}
