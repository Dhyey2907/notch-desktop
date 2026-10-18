// Price watch: product links checked every few hours; a card when one drops to your target.
// Not in the dock: the palette ("Price watch", "watch <link>") opens it.
import { Tag } from "lucide-react";
import { lazy } from "react";

import { ipc } from "../../core/ipc";
import type { ModuleContext, NotchModule } from "../types";
import { usePrices } from "./pricesStore";

function init({ events, activities }: ModuleContext) {
  const offChanged = events.on("PricesChanged", ({ event }) => usePrices.getState().set(event.payload));
  const offDrop = events.on("PriceDrop", ({ event }) => {
    const { title, price, target, currency, url } = event.payload;
    const sym = currency === "INR" ? "₹" : currency === "USD" ? "$" : currency;
    activities.push({
      moduleId: "prices",
      priority: "normal",
      tone: "success",
      dedupKey: `price-${url}`,
      countRepeats: false,
      persistent: true,
      title: `Price drop: ${title || "something you watch"}`,
      body: `Now ${sym}${price.toLocaleString()}, at or under your ${sym}${target.toLocaleString()} target.`,
      actions: [
        { id: "dismiss", label: "Later", variant: "secondary", run: () => {} },
        { id: "open", label: "Open", variant: "primary", run: () => ipc.openExternal(url) },
      ],
    });
  });
  ipc.priceList().then(
    (items) => usePrices.getState().set(items),
    () => {},
  );
  return () => {
    offChanged();
    offDrop();
  };
}

export const pricesModule: NotchModule = {
  id: "prices",
  name: "Price watch",
  icon: Tag,
  order: 995,
  alwaysOn: true,
  dockless: true,
  init,
  Expanded: lazy(() => import("./PricesExpanded")),
};
