// Pet: your companion up close — pat it, see what it's up to, hear it talk, and watch it grow.
import { PawPrint } from "lucide-react";
import { lazy } from "react";

import type { NotchModule } from "../types";

export const petModule: NotchModule = {
  id: "pet",
  name: "Pet",
  icon: PawPrint,
  order: 70,
  Expanded: lazy(() => import("./PetExpanded")),
};
