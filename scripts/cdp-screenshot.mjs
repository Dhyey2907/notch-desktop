// Dev helper: capture the running Tauri webview to a PNG over the WebView2 DevTools protocol.
// Usage: node scripts/cdp-screenshot.mjs <out.png>   (see scripts/cdp-eval.mjs for setup)
import { writeFile } from "node:fs/promises";

const port = process.env.CDP_PORT ?? "9222";
const out = process.argv[2] ?? "notch.png";

const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
// CDP_WINDOW=settings targets the settings window; otherwise the island.
const wantSettings = process.env.CDP_WINDOW === "settings";
// CDP_INDEX=n picks the n-th matching island when several screens have one.
const matches = targets.filter(
  (t) =>
    t.type === "page" && t.url.includes("localhost:1420") && t.url.includes("#settings") === wantSettings,
);
const page = matches[Number(process.env.CDP_INDEX ?? 0)] ?? matches[0];
if (!page) {
  console.error("no Overline webview found");
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
ws.onmessage = async (message) => {
  const data = JSON.parse(message.data);
  if (data.id !== 1) return;
  await writeFile(out, Buffer.from(data.result.data, "base64"));
  console.log(out);
  ws.close();
};
ws.send(JSON.stringify({ id: 1, method: "Page.captureScreenshot", params: { format: "png" } }));
