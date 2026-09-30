// Which "Continue with …" sign-ins this build offers (asked once per window).
import { useEffect, useState } from "react";

import type { SignInOptions } from "../core/generated/SignInOptions";
import { ipc } from "../core/ipc";

let cached: Promise<SignInOptions> | null = null;

export function useSignInOptions(): SignInOptions | null {
  const [options, setOptions] = useState<SignInOptions | null>(null);
  useEffect(() => {
    cached ??= ipc.signInOptions();
    let alive = true;
    cached.then(
      (o) => alive && setOptions(o),
      () => alive && setOptions({ google: false, spotify: false, github: false }),
    );
    return () => {
      alive = false;
    };
  }, []);
  return options;
}
