// Price watch: product links with a target price. Each is checked every few hours; a card says
// when one drops to the target. A tiny line shows how the price moved.
import { ExternalLink, RefreshCw, Trash2 } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";

import type { PriceItem } from "../../core/generated/PriceItem";
import { ipc, isIpcError } from "../../core/ipc";
import { Button, EmptyState, IconButton, ModuleContainer } from "../../design/components";
import { usePrices } from "./pricesStore";
import "./prices.css";

function money(v: number | undefined, currency: string): string {
  if (v === undefined) return "–";
  const sym = currency === "INR" ? "₹" : currency === "USD" ? "$" : currency === "EUR" ? "€" : currency;
  return `${sym}${v.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
}

/** A small line of the price history (oldest → newest). */
function Spark({ points }: { points: [number, number][] }) {
  if (points.length < 2) return null;
  const ps = points.map(([, p]) => p);
  const lo = Math.min(...ps);
  const hi = Math.max(...ps);
  const d = ps
    .map((p, i) => {
      const x = (i / (ps.length - 1)) * 60;
      const y = hi === lo ? 9 : 16 - ((p - lo) / (hi - lo)) * 14;
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <svg className="prices__spark" viewBox="0 0 60 18" aria-hidden>
      <path d={d} />
    </svg>
  );
}

export default function PricesExpanded() {
  const items = usePrices((s) => s.items);
  const [url, setUrl] = useState("");
  const [target, setTarget] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc.priceList().then(
      (l) => usePrices.getState().set(l),
      () => {},
    );
  }, []);

  const msg = (e: unknown) =>
    isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "That didn't work";

  const add = (e: FormEvent) => {
    e.preventDefault();
    const t = Number(target.replace(/[^\d.]/g, ""));
    if (!url.trim() || !(t > 0)) {
      setError("Paste a product link and a target price");
      return;
    }
    setBusy("add");
    void ipc
      .priceAdd(url.trim(), t)
      .then(
        () => {
          setUrl("");
          setTarget("");
          setError(null);
          return ipc.priceList().then((l) => usePrices.getState().set(l));
        },
        (err) => setError(msg(err)),
      )
      .finally(() => setBusy(null));
  };

  const check = (item: PriceItem) => {
    setBusy(item.id);
    void ipc
      .priceCheck(item.id)
      .catch((err) => setError(msg(err)))
      .finally(() => setBusy(null));
  };

  return (
    <ModuleContainer title="Price watch">
      <div className="prices">
        <form className="prices__add" onSubmit={add}>
          <input
            className="nd-input"
            aria-label="Product link"
            placeholder="Paste a product link (Amazon, Flipkart, any shop)"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
          />
          <input
            className="nd-input prices__target"
            aria-label="Target price"
            placeholder="Target"
            inputMode="decimal"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
          />
          <Button type="submit" variant="primary" disabled={busy === "add"}>
            {busy === "add" ? "Checking…" : "Watch"}
          </Button>
        </form>
        {error && <p className="prices__error">{error}</p>}
        {items.length === 0 ? (
          <EmptyState
            title="No prices watched"
            body="Add a product and the price you'd pay. It's checked every few hours, and you get a card when it drops."
          />
        ) : (
          <ul className="prices__list">
            {items.map((i) => {
              const hit = i.price !== undefined && i.price <= i.target;
              return (
                <li key={i.id} className={`prices__item${hit ? " prices__item--hit" : ""}`}>
                  <span className="prices__title" title={i.url}>
                    {i.title || new URL(i.url).hostname}
                  </span>
                  <Spark points={i.history} />
                  <span className="prices__now" title={i.error ?? undefined}>
                    {i.error && i.price === undefined ? "couldn't read" : money(i.price, i.currency)}
                  </span>
                  <span className="prices__goal">≤ {money(i.target, i.currency)}</span>
                  <IconButton
                    icon={RefreshCw}
                    label="Check now"
                    size="sm"
                    disabled={busy === i.id}
                    onClick={() => check(i)}
                  />
                  <IconButton
                    icon={ExternalLink}
                    label="Open the page"
                    size="sm"
                    onClick={() => void ipc.openExternal(i.url).catch(() => {})}
                  />
                  <IconButton
                    icon={Trash2}
                    label="Stop watching"
                    size="sm"
                    onClick={() => void ipc.priceRemove(i.id).catch((err) => setError(msg(err)))}
                  />
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </ModuleContainer>
  );
}
