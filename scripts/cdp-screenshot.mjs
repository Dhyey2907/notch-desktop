// Dev helper: capture the running Tauri webview to a PNG over the WebView2 DevTools protocol.
// Usage: node scripts/cdp-screenshot.mjs <out.png>   (see scripts/cdp-eval.mjs for setup)
import { writeFile } from "node:fs/promises";

const port = process.env.CDP_PORT ?? "9222";
const out = process.argv[2] ?? "notch.png";

const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const page = targets.find((t) => t.type === "page" && t.url.includes("localhost:1420"));
if (!page) {
  console.error("no Notch Desktop webview found");
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
