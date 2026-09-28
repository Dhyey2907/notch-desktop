import { MotionConfig } from "motion/react";

import { Island } from "./shell/Island";

function App() {
  return (
    // Respect the OS "reduce motion" preference everywhere (architecture.md §4).
    <MotionConfig reducedMotion="user">
      <Island />
    </MotionConfig>
  );
}

export default App;
