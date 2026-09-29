import { useEffect, useState } from "react";

import { ipc } from "../../core/ipc";
import { EmptyState, ModuleContainer, Pill } from "../../design/components";

export default function HomeExpanded() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .appInfo()
      .then((info) => setVersion(info.version))
      .catch(() => setVersion(null));
  }, []);

  return (
    <ModuleContainer title="Notch Desktop" trailing={version && <Pill>v{version}</Pill>}>
      <EmptyState
        title="Everything in one place"
        body="Pick Media, Calendar, Git or Claude Code from the dock below. Press Ctrl+Alt+N anytime to open or close the notch."
      />
    </ModuleContainer>
  );
}
