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
// CDP_WINDOW=settings targets the settings window; otherwise the island.
// CDP_ALL=1 evaluates in every island (one per screen) and prefixes each result.
const wantSettings = process.env.CDP_WINDOW === "settings";
const pages = targets.filter(
  (t) =>
    t.type === "page" && t.url.includes("localhost:1420") && t.url.includes("#settings") === wantSettings,
);
if (pages.length === 0) {
  console.error(
    "no Overline webview found; targets:",
    targets.map((t) => t.url),
  );
  process.exit(1);
}

async function evaluate(page) {
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  const reply = new Promise((resolve) => {
    ws.onmessage = (message) => {
      const data = JSON.parse(message.data);
      if (data.id === 1) resolve(data.result);
    };
    // The window can close mid-evaluation (e.g. a setting that removes that island).
    ws.onclose = () => resolve({ result: { value: "(window closed)" } });
  });
  ws.send(
    JSON.stringify({
      id: 1,
      method: "Runtime.evaluate",
      params: { expression, awaitPromise: true, returnByValue: true },
    }),
  );
  const { result, exceptionDetails } = await reply;
  ws.close();
  if (exceptionDetails) {
    process.exitCode = 1;
    return exceptionDetails.exception?.description ?? exceptionDetails.text;
  }
  return typeof result.value === "string" ? result.value : JSON.stringify(result.value, null, 2);
}

if (process.env.CDP_ALL === "1") {
  for (const [i, page] of pages.entries()) console.log(`[${i}] ${await evaluate(page)}`);
} else {
  console.log(await evaluate(pages[0]));
}
