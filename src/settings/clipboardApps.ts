/** "Slack" or "C:\...\slack.exe" -> "slack.exe" (how the block list matches apps). */
export function exeName(input: string): string {
  const name = input.trim().split(/[\\/]/).pop()?.trim().toLowerCase() ?? "";
  return !name || name.endsWith(".exe") ? name : `${name}.exe`;
}
