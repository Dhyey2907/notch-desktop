// The privacy cover (#blur): a softened picture of this screen filling it, until a click,
// Esc or the shortcut again takes every cover away.
import { invoke } from "@tauri-apps/api/core";
import { EyeOff } from "lucide-react";
import { useEffect, useState } from "react";

import "./blur.css";

export function BlurCover() {
  const [picture, setPicture] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("blur_picture").then(setPicture, () => {});
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void invoke("blur_off");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <button
      type="button"
      className="blur-cover"
      aria-label="Show the screen again"
      onClick={() => void invoke("blur_off")}
    >
      {picture && <img className="blur-cover__picture" src={picture} alt="" draggable={false} />}
      <span className="blur-cover__note">
        <EyeOff size={16} aria-hidden /> Hidden · click or press Esc
      </span>
    </button>
  );
}
