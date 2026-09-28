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
        title="Your modules live here"
        body="Media, Calendar, Git and Claude will join the dock below as they are built. Press Ctrl+Alt+N anytime to open or close the notch."
      />
    </ModuleContainer>
  );
}
