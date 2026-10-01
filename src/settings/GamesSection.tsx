import { Download, X } from "lucide-react";
import { useEffect, useState } from "react";

import type { GameStatus } from "../core/generated/GameStatus";
import { ipc, isIpcError } from "../core/ipc";
import { Button, IconButton, Surface, Switch } from "../design/components";
import { useSettings } from ".";
import { pageLead } from "./searchIndex";
import { Section } from "./Section";

export function GamesSection() {
  const game = useSettings((s) => s.settings?.game);
  const [status, setStatus] = useState<GameStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc.gameStatus().then(setStatus, () => {});
  }, []);

  const save = (change: { games?: string[]; notGames?: string[]; disabled?: boolean }) =>
    void useSettings.getState().update((s) => ({ ...s, game: { ...s.game, ...change } }));

  const setup = async () => {
    setBusy(true);
    setError(null);
    try {
      await ipc.gameSetupFps();
      setStatus(await ipc.gameStatus());
    } catch (e) {
      setError(isIpcError(e) ? e.message.replace(/^invalid input: /, "") : "Couldn't set up FPS.");
    } finally {
      setBusy(false);
    }
  };

  if (!game) return null;
  return (
    <Section title="Games" lead={pageLead("games")}>
      <Surface>
        <Switch
          label="Game mode"
          description="When a game you've chosen plays fullscreen, the notch shows FPS, GPU temperature and load, CPU, RAM, ping and the time, and holds notifications until you're done."
          checked={!game.disabled}
          onChange={(on) => save({ disabled: !on })}
        />
      </Surface>
      <Surface>
        <div className="settings__row">
          <span className="settings__row-text">
            <span className="settings__row-label">FPS counter</span>
            <span className="settings__row-description">
              {status?.fpsReady
                ? "Ready — FPS comes from Intel's open-source PresentMon."
                : "Downloads PresentMon (about 1 MB, from Intel's official GitHub release) and checks its fingerprint before using it."}
            </span>
          </span>
          {!status?.fpsReady && (
            <Button variant="primary" disabled={busy} onClick={() => void setup()}>
              <Download size={14} aria-hidden /> {busy ? "Setting up…" : "Set up FPS"}
            </Button>
          )}
        </div>
        {error && <p className="settings__note settings__note--warning">{error}</p>}
      </Surface>
      <Surface>
        <p className="settings__row-label">Your games</p>
        {game.games.length === 0 ? (
          <p className="settings__note">
            None yet. When an app goes fullscreen, Notch asks once whether it's a game.
          </p>
        ) : (
          <ul className="settings__chips" aria-label="Games">
            {game.games.map((g) => (
              <li key={g} className="settings__chip">
                {g}
                <IconButton
                  icon={X}
                  label={`Stop using game mode for ${g}`}
                  size="sm"
                  onClick={() => save({ games: game.games.filter((x) => x !== g) })}
                />
              </li>
            ))}
          </ul>
        )}
        {game.notGames.length > 0 && (
          <p className="settings__note">
            Not games: {game.notGames.join(", ")}{" "}
            <button type="button" className="home-link" onClick={() => save({ notGames: [] })}>
              Ask again
            </button>
          </p>
        )}
      </Surface>
    </Section>
  );
}
