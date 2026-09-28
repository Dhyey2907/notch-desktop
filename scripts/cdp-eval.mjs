// Dev helper: evaluate JavaScript in the running Tauri webview over the WebView2
// DevTools protocol. Start the app with
//   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 pnpm tauri dev
// then: node scripts/cdp-eval.mjs "<expression>"   (async expressions are awaited)
//   or: node scripts/cdp-eval.mjs path/to/snippet.js
import { existsSync, readFileSync } from "node:fs";

const port = process.env.CDP_PORT ?? "9222";
const arg = process.argv[2];
if (!arg) {
  console.error('usage: node scripts/cdp-eval.mjs "<expression>" | <file.js>');
  process.exit(2);
}
const expression = arg.endsWith(".js") && existsSync(arg) ? readFileSync(arg, "utf8") : arg;

const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const page = targets.find((t) => t.type === "page" && t.url.includes("localhost:1420"));
if (!page) {
  console.error(
    "no Notch Desktop webview found; targets:",
    targets.map((t) => t.url),
  );
  process.exit(1);
}

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
ws.onmessage = (message) => {
  const data = JSON.parse(message.data);
  if (data.id !== 1) return;
  const { result, exceptionDetails } = data.result;
  if (exceptionDetails) {
    console.error(exceptionDetails.exception?.description ?? exceptionDetails.text);
    process.exitCode = 1;
  } else {
    console.log(typeof result.value === "string" ? result.value : JSON.stringify(result.value, null, 2));
  }
  ws.close();
};
ws.send(
  JSON.stringify({
    id: 1,
    method: "Runtime.evaluate",
    params: { expression, awaitPromise: true, returnByValue: true },
  }),
);
