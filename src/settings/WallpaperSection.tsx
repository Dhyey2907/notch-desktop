import { FolderOpen } from "lucide-react";
import { useState } from "react";

import { ipc, isIpcError } from "../core/ipc";
import { Button, ConfirmButton, Surface } from "../design/components";
import { useSettings } from ".";
import { Row } from "./controls";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

function message(e: unknown) {
  return isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "That didn't work.";
}

export function WallpaperSection() {
  const wallpaper = useSettings((s) => s.settings?.wallpaper);
  const [note, setNote] = useState<string | null>(null);
  if (!wallpaper) return null;

  return (
    <Section title="Wallpaper" lead={pageLead("wallpaper")}>
      <Surface>
        <div id="setting-wallpaper-folder">
          <Row
            label="Wallpaper folder"
            description={
              wallpaper.folder
                ? `"My folder" shows the pictures in ${wallpaper.folder}.`
                : 'Pick a folder and every picture in it shows up under "My folder" in the Wallpaper panel.'
            }
          >
            <Button onClick={() => void ipc.wallpaperPickFolder().catch((e) => setNote(message(e)))}>
              <FolderOpen size={14} aria-hidden /> {wallpaper.folder ? "Change" : "Choose folder"}
            </Button>
            {wallpaper.folder && (
              <Button onClick={() => void ipc.wallpaperSetFolder(null).catch((e) => setNote(message(e)))}>
                Forget
              </Button>
            )}
          </Row>
        </div>
        <div id="setting-recent-wallpapers">
          <Row
            label="Recent wallpapers"
            description={`${wallpaper.recent.length} remembered. Saved photos are kept in Overline's own folder until you remove them.`}
          >
            <ConfirmButton
              confirmLabel="Clear recent?"
              disabled={wallpaper.recent.length === 0}
              onConfirm={() => void ipc.wallpaperClearRecent().catch((e) => setNote(message(e)))}
            >
              Clear
            </ConfirmButton>
          </Row>
        </div>
        {note && <p className="settings__note">{note}</p>}
      </Surface>
    </Section>
  );
}
