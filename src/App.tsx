import { MotionConfig } from "motion/react";

import { useSettings } from "./settings";
import { Island } from "./shell/Island";

function App() {
  const animation = useSettings((s) => s.settings?.animation);
  return (
    // Follow Windows' animation preference, or force reduced motion if chosen in Settings.
    <MotionConfig reducedMotion={animation === "reduced" ? "always" : "user"}>
      <Island />
    </MotionConfig>
  );
}

export default App;
