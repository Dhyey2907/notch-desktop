// "Continue with Google / Spotify / GitHub" buttons. They appear only when this build has
// Notch's own registered OAuth clients (src-tauri/oauth-clients.json); otherwise the existing
// "paste your own client ID" setup is shown on its own.
import type { ReactNode } from "react";

import type { SignInOptions } from "../core/generated/SignInOptions";
import { useSignInOptions } from "./signInOptions";

export type Provider = keyof SignInOptions;

const NAMES: Record<Provider, string> = { google: "Google", spotify: "Spotify", github: "GitHub" };

function Logo({ provider }: { provider: Provider }) {
  if (provider === "google") {
    return (
      <svg viewBox="0 0 48 48" width="18" height="18" aria-hidden>
        <path
          fill="#EA4335"
          d="M24 9.5c3.5 0 6.6 1.2 9.1 3.6l6.8-6.8C35.8 2.4 30.3 0 24 0 14.6 0 6.6 5.4 2.6 13.3l7.9 6.2C12.4 13.6 17.7 9.5 24 9.5z"
        />
        <path
          fill="#4285F4"
          d="M46.1 24.5c0-1.6-.1-3.1-.4-4.5H24v9h12.4c-.5 2.9-2.2 5.3-4.6 6.9l7.2 5.6c4.2-3.9 7.1-9.6 7.1-17z"
        />
        <path
          fill="#FBBC05"
          d="M10.5 28.6c-.5-1.4-.8-3-.8-4.6s.3-3.2.8-4.6l-7.9-6.2C1 16.6 0 20.2 0 24s1 7.4 2.6 10.8l7.9-6.2z"
        />
        <path
          fill="#34A853"
          d="M24 48c6.5 0 11.9-2.1 15.9-5.8l-7.2-5.6c-2 1.4-4.7 2.3-8.7 2.3-6.3 0-11.6-4.1-13.5-9.9l-7.9 6.2C6.6 42.6 14.6 48 24 48z"
        />
      </svg>
    );
  }
  if (provider === "spotify") {
    return (
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden>
        <circle cx="12" cy="12" r="12" fill="#1ED760" />
        <path
          fill="none"
          stroke="#000"
          strokeLinecap="round"
          strokeWidth="1.8"
          d="M6.5 9.3c3.7-1.1 8-.8 11 1M7.3 12.6c3-.8 6.4-.5 8.9.9M8 15.7c2.4-.6 4.9-.4 6.8.7"
        />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden>
      <path
        fill="currentColor"
        d="M12 .3a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2c-3.3.7-4-1.6-4-1.6-.6-1.4-1.4-1.8-1.4-1.8-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-6 0-1.2.5-2.3 1.2-3.1-.1-.4-.5-1.6.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C17 4.7 18 5 18 5c.6 1.6.2 2.8.1 3.2.8.8 1.2 1.9 1.2 3.1 0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .3"
      />
    </svg>
  );
}

/** Branded "Continue with …" button. */
export function ProviderButton({
  provider,
  label,
  disabled,
  onClick,
}: {
  provider: Provider;
  label?: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`signin-button signin-button--${provider}`}
      disabled={disabled}
      onClick={onClick}
    >
      <Logo provider={provider} />
      <span>{label ?? `Continue with ${NAMES[provider]}`}</span>
    </button>
  );
}

/**
 * One-click sign-in when this build supports it, with the manual setup (`children`) tucked
 * into "Use your own … client instead"; otherwise just the manual setup.
 */
export function SignInWith({
  provider,
  busy,
  reconnect,
  error,
  onContinue,
  children,
}: {
  provider: Provider;
  busy: boolean;
  reconnect?: boolean;
  error?: string | null;
  onContinue: () => void;
  children: ReactNode;
}) {
  const options = useSignInOptions();
  if (!options) return null;
  if (!options[provider]) return <>{children}</>;
  const name = NAMES[provider];
  return (
    <>
      <div className="settings__buttons settings__buttons--start">
        <ProviderButton
          provider={provider}
          disabled={busy}
          label={busy ? "Waiting for you in the browser…" : reconnect ? `Reconnect with ${name}` : undefined}
          onClick={onContinue}
        />
      </div>
      {error && <p className="settings__note settings__note--warning">{error}</p>}
      <details className="settings__details">
        <summary>Use your own {name} client instead</summary>
        {children}
      </details>
    </>
  );
}
