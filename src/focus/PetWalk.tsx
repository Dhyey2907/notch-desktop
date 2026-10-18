// The pet's walk along the taskbar (the transparent #pet window). It patrols, and you can play
// with it: click for a treat, double-click to play fetch, rub it to pet it, swipe through it to
// make it dizzy, pick it up and drop it, or rest the cursor nearby and it sneaks up and pounces.
// It cheers when you get things done, dances to your music, wears what the weather calls for,
// and a friend drops by now and then. When you come back and click, Notch sends "pet-leave":
// it jumps, then runs off the nearest edge.
//
// The window lets clicks through to the taskbar. Notch tells it where the mouse is
// ("pet-pointer"); while the pointer is on the pet, the window takes the mouse so the pet can be
// clicked and carried.
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";

import { eventBus } from "../core/eventBus";
import {
  WALK_DID_EVENT,
  WALK_HELLO_EVENT,
  WALK_LOOK_EVENT,
  WALK_PLAY_EVENT,
  WALK_TREAT_EVENT,
  type WalkDid,
  type WalkLook,
  type WalkPlay,
} from "../pet/useWalkLook";
import { useSettings } from "../settings";
import { classify, onPet, recent, restingAt, swipedThrough, type Sample } from "./petGestures";
import { carry, drop, PET_W, runTo, startBody, step, type Body } from "./petMotion";
import { WalkerFigure } from "./WalkerFigure";
import "./pet.css";

/** How long a line stays in the cloud (unless it asks for longer, like a reminder). */
const CLOUD_MS = 9000;
const TREAT_MS = 2600;
/** A click this close to the pet still counts as a click on it (it's small and it moves). */
const CLICK_SLOP_PX = 10;
/** Closer to a screen edge than this, the cloud opens towards the middle so it isn't cut off. */
const CLOUD_EDGE_PX = 150;
/** A second click this soon makes it a double-click (fetch), not two treats. */
const DOUBLE_CLICK_MS = 260;
const DRAG_START_PX = 6;
/** How high you can lift it (its feet above the taskbar). */
const LIFT_MAX = 96;
/** It won't chase your cursor more often than this. */
const CHASE_COOLDOWN_MS = 5 * 60_000;
const FRIEND_MIN_MS = 25 * 60_000;
const FRIEND_MAX_MS = 55 * 60_000;
const BALL_GRAVITY = 1400;

type Side = "left" | "center" | "right";
type Fx =
  "purr" | "dizzy" | "pounce" | "cheer" | "stretch" | "flail" | "backflip" | "juggle" | "wave" | "treat";

interface Ball {
  x: number;
  y: number;
  vx: number;
  vy: number;
  state: "flying" | "ground" | "carried";
}

interface Friend {
  x: number;
  dir: 1 | -1;
  waveUntil: number;
  waved: boolean;
}

function cloudSide(x: number, screenWidth: number, edge = CLOUD_EDGE_PX): Side {
  if (x < edge) return "left";
  if (x > screenWidth - edge) return "right";
  return "center";
}

const did = (what: WalkDid["what"]) => void emitTo("main", WALK_DID_EVENT, { what }).catch(() => {});

export function PetWalk() {
  const track = useRef<HTMLDivElement>(null);
  const petEl = useRef<HTMLDivElement>(null);
  const ballEl = useRef<HTMLSpanElement>(null);
  const friendEl = useRef<HTMLDivElement>(null);
  const color = useSettings((s) => s.settings?.pet.color) || undefined;
  const play = useSettings((s) => s.settings?.pet.play ?? true);
  const friends = useSettings((s) => s.settings?.pet.friends ?? true);
  const reactMusic = useSettings((s) => s.settings?.pet.reactMusic ?? true);
  const [look, setLook] = useState<WalkLook>({ face: "happy", prop: "none", line: null });
  const [cloud, setCloud] = useState<{ text: string; at: number; side: Side } | null>(null);
  const [fx, setFx] = useState<{ kind: Fx; at: number; until: number } | null>(null);
  const [leaving, setLeaving] = useState<{ dir: 1 | -1 } | null>(null);
  const [ballOn, setBallOn] = useState(false);
  const [friendOn, setFriendOn] = useState(false);

  // Everything the frame loop reads, kept out of React so moving doesn't re-render.
  const body = useRef<Body>(startBody(window.innerWidth));
  const ball = useRef<Ball | null>(null);
  const fetch = useRef<{ phase: "chase" | "return"; home: number } | null>(null);
  const chase = useRef(false);
  const lastChase = useRef(0);
  const samples = useRef<Sample[]>([]);
  const captured = useRef(false);
  const press = useRef<{ x: number; y: number; dragging: boolean; grab: number } | null>(null);
  const clickTimer = useRef<number | undefined>(undefined);
  const friend = useRef<Friend | null>(null);
  const lastGesture = useRef(0);
  const leaveDir = useRef<1 | -1>(1);
  const sulky = useRef(false);
  const playOn = useRef(play);
  /** Standing still for a moment (after a pounce). */
  const holdStill = useRef(false);
  const leavingRef = useRef(false);
  useEffect(() => {
    sulky.current = !!look.sulky;
  }, [look.sulky]);
  useEffect(() => {
    playOn.current = play;
  }, [play]);
  useEffect(() => {
    leavingRef.current = leaving !== null;
  }, [leaving]);

  const effect = (kind: Fx, ms: number) => {
    const now = Date.now();
    setFx({ kind, at: now, until: now + ms });
  };

  useEffect(() => {
    if (!fx) return;
    const id = window.setTimeout(() => setFx(null), Math.max(0, fx.until - Date.now()));
    return () => window.clearTimeout(id);
  }, [fx]);

  const capture = (on: boolean) => {
    if (captured.current === on) return;
    captured.current = on;
    void invoke("pet_capture", { on }).catch(() => {});
  };

  const treat = () => {
    effect("treat", TREAT_MS);
    void emitTo("main", WALK_TREAT_EVENT, null).catch(() => {});
  };

  const startFetch = () => {
    if (leaving) return;
    const b = body.current;
    const fromX = b.x + PET_W / 2;
    const toRight = fromX < window.innerWidth / 2;
    ball.current = {
      x: fromX,
      y: 26,
      vx: (toRight ? 1 : -1) * (260 + Math.random() * 220),
      vy: 380 + Math.random() * 140,
      state: "flying",
    };
    fetch.current = { phase: "chase", home: b.x };
    chase.current = false;
    setBallOn(true);
  };

  // Handlers the long-lived listeners below call (always the latest ones).
  const actions = useRef({ treat, startFetch });
  useEffect(() => {
    actions.current = { treat, startFetch };
  });

  // The frame loop: move the pet, the ball and a visiting friend.
  useEffect(() => {
    let raf = 0;
    let last = performance.now();
    let lastRestCheck = 0;
    const frame = (t: number) => {
      const dt = Math.min(0.05, (t - last) / 1000);
      last = t;
      const width = window.innerWidth;
      const now = Date.now();

      // The ball flies, bounces and rolls to a stop.
      const bl = ball.current;
      if (bl && bl.state !== "carried") {
        if (bl.state === "flying") {
          bl.vy -= BALL_GRAVITY * dt;
          bl.y += bl.vy * dt;
          bl.x += bl.vx * dt;
          if (bl.x < 10 || bl.x > width - 10) {
            bl.vx = -bl.vx * 0.6;
            bl.x = Math.min(Math.max(bl.x, 10), width - 10);
          }
          if (bl.y <= 4) {
            bl.y = 4;
            if (Math.abs(bl.vy) > 120) bl.vy = -bl.vy * 0.45;
            else {
              bl.vy = 0;
              bl.vx *= 0.92;
              if (Math.abs(bl.vx) < 12) bl.state = "ground";
            }
          }
        }
      }
      const fe = fetch.current;
      if (fe && bl && fe.phase === "chase") body.current = runTo(body.current, bl.x - PET_W / 2);

      if (leavingRef.current) {
        body.current = {
          ...body.current,
          x: body.current.x + leaveDir.current * 420 * dt,
          dir: leaveDir.current,
        };
      } else if (!press.current?.dragging && !holdStill.current) {
        const r = step(body.current, dt, width, { sulky: sulky.current });
        body.current = r.body;
        if (r.landed) did("dropped");
        if (r.arrived) {
          if (fe && bl && fe.phase === "chase") {
            if (bl.state === "ground" || Math.abs(bl.vx) < 40) {
              bl.state = "carried";
              fe.phase = "return";
              body.current = runTo(body.current, fe.home);
            }
          } else if (fe && fe.phase === "return") {
            fetch.current = null;
            ball.current = null;
            setBallOn(false);
            body.current = { ...body.current, mode: "walk" };
            effect("cheer", 1100);
            did("fetched");
          } else if (chase.current) {
            chase.current = false;
            holdStill.current = true;
            effect("pounce", 750);
            did("pounce");
            window.setTimeout(() => {
              holdStill.current = false;
              body.current = { ...body.current, mode: "walk" };
            }, 750);
          } else {
            body.current = { ...body.current, mode: "walk" };
          }
        }
      }

      // Sneak up on a cursor resting on the taskbar strip (not on the pet itself).
      if (
        t - lastRestCheck > 250 &&
        playOn.current &&
        !leavingRef.current &&
        !fetch.current &&
        !chase.current &&
        (body.current.mode === "walk" || body.current.mode === "pause") &&
        now - lastChase.current > CHASE_COOLDOWN_MS
      ) {
        lastRestCheck = t;
        const spot = restingAt(samples.current, now);
        const r = petEl.current?.getBoundingClientRect();
        const last = samples.current[samples.current.length - 1];
        if (spot !== null && r && last && !onPet(last, r, 30)) {
          lastChase.current = now;
          chase.current = true;
          body.current = runTo(body.current, spot - PET_W / 2);
        }
      }

      // A friend walks over, they wave, and it walks on.
      const fr = friend.current;
      if (fr) {
        if (fr.waveUntil > now) {
          // waving
        } else {
          fr.x += fr.dir * 60 * dt;
          if (!fr.waved && Math.abs(fr.x - body.current.x) < 44) {
            fr.waved = true;
            fr.waveUntil = now + 1700;
            effect("wave", 1700);
            did("friend");
          }
          if (fr.x < -60 || fr.x > width + 60) {
            friend.current = null;
            setFriendOn(false);
          }
        }
        if (friendEl.current) {
          friendEl.current.style.transform = `translateX(${fr.x}px) scaleX(${fr.dir})`;
        }
      }

      const b = body.current;
      if (track.current) track.current.style.transform = `translate(${b.x}px, ${-b.y}px)`;
      if (petEl.current && !leavingRef.current) {
        petEl.current.style.setProperty("--face-dir", String(b.dir));
        petEl.current.dataset.mode = b.mode;
      }
      const bl2 = ball.current;
      if (ballEl.current && bl2) {
        const bx = bl2.state === "carried" ? b.x + (b.dir === 1 ? PET_W - 4 : -4) : bl2.x - 5;
        const by = bl2.state === "carried" ? b.y + 9 : bl2.y;
        ballEl.current.style.transform = `translate(${bx}px, ${-by}px)`;
      }
      raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }, []);

  // Where the mouse is over the strip (from Notch): petting, swipes, chasing, and when to take
  // the mouse so the pet can be clicked and carried.
  useEffect(() => {
    const off = listen<{ x: number; y: number; down: boolean; inside: boolean }>(
      "pet-pointer",
      ({ payload }) => {
        const now = Date.now();
        // The button is up but the window never heard it (it lost the mouse mid-carry): drop it.
        if (press.current && payload.inside && !payload.down) {
          const lost = press.current;
          press.current = null;
          if (lost.dragging) {
            body.current = drop(body.current);
            setFx(null);
          }
        }
        if (!payload.inside) {
          samples.current = [];
          if (!press.current) capture(false);
          return;
        }
        samples.current = recent([...samples.current, { x: payload.x, y: payload.y, t: now }], now, 3000);
        const r = petEl.current?.getBoundingClientRect();
        if (!r || leavingRef.current) return;
        const over = onPet(payload, r);
        if (playOn.current && over) capture(true);
        else if (!over && !press.current) capture(false);
        if (!playOn.current || now - lastGesture.current < 2500) return;
        const onIt = recent(samples.current, now).filter((s) => onPet(s, r, 14));
        const gesture = swipedThrough(recent(samples.current, now, 400), r) ? "swipe" : classify(onIt);
        if (gesture === "pat") {
          lastGesture.current = now;
          effect("purr", 1800);
          did("pat");
        } else if (gesture === "swipe") {
          lastGesture.current = now;
          effect("dizzy", 2000);
          did("dizzy");
        }
      },
    );
    return () => void off.then((f) => f());
  }, []);

  // Clicks while the window lets the mouse through come from Notch: on the pet = a treat.
  useEffect(() => {
    const off = listen<{ x: number; y: number }>("pet-click", ({ payload }) => {
      const r = petEl.current?.getBoundingClientRect();
      if (!r || !onPet(payload, r, CLICK_SLOP_PX)) return;
      actions.current.treat();
    });
    return () => void off.then((f) => f());
  }, []);

  // Fetch, a cheer, waking up, or a trick: asked for by the island.
  useEffect(() => {
    const off = listen<WalkPlay>(WALK_PLAY_EVENT, ({ payload }) => {
      if (leavingRef.current) return;
      if (payload.kind === "fetch") actions.current.startFetch();
      else if (payload.kind === "celebrate") effect("cheer", 1500);
      else if (payload.kind === "wake") effect("stretch", 1900);
      else if (payload.kind === "trick") effect(payload.trick === "juggle" ? "juggle" : "backflip", 1600);
    });
    return () => void off.then((f) => f());
  }, []);

  // It bobs to the beat of your music.
  useEffect(() => {
    if (!reactMusic) return;
    let lastBeat = 0;
    return eventBus.on("MusicPulse", ({ event }) => {
      if (!event.payload.beat || !petEl.current) return;
      const now = performance.now();
      if (now - lastBeat < 180) return;
      lastBeat = now;
      const el = petEl.current;
      const big = event.payload.level > 0.75;
      el.classList.remove("petwalk__pet--beat", "petwalk__pet--hop");
      void el.offsetWidth;
      el.classList.add(big ? "petwalk__pet--hop" : "petwalk__pet--beat");
    });
  }, [reactMusic]);

  // A friend drops by now and then while it's out.
  useEffect(() => {
    if (!friends) return;
    let timer: number;
    const plan = () => {
      timer = window.setTimeout(
        () => {
          if (!leavingRef.current && !friend.current) {
            const fromLeft = body.current.x > window.innerWidth / 2;
            friend.current = {
              x: fromLeft ? -50 : window.innerWidth + 50,
              dir: fromLeft ? 1 : -1,
              waveUntil: 0,
              waved: false,
            };
            setFriendOn(true);
          }
          plan();
        },
        FRIEND_MIN_MS + Math.random() * (FRIEND_MAX_MS - FRIEND_MIN_MS),
      );
    };
    plan();
    return () => window.clearTimeout(timer);
  }, [friends]);

  // What it looks like and says comes from the island.
  useEffect(() => {
    const off = listen<WalkLook>(WALK_LOOK_EVENT, ({ payload }) => setLook(payload));
    void emitTo("main", WALK_HELLO_EVENT, null).catch(() => {});
    return () => void off.then((f) => f());
  }, []);

  // A new line: show it in the cloud for a few seconds, opening away from the screen edge.
  const lineText = look.line?.text;
  const lineAt = look.line?.at ?? 0;
  const lineHold = look.line?.holdMs ?? CLOUD_MS;
  useEffect(() => {
    const left = lineAt + lineHold - Date.now();
    if (!lineText || left <= 0) return;
    const show = window.setTimeout(
      () => setCloud({ text: lineText, at: lineAt, side: cloudSide(body.current.x, window.innerWidth) }),
      0,
    );
    const hide = window.setTimeout(() => setCloud(null), left);
    return () => {
      window.clearTimeout(show);
      window.clearTimeout(hide);
    };
  }, [lineText, lineAt, lineHold]);

  // Going back into the notch: jump, then run off the closer edge.
  useEffect(() => {
    const unlisten = listen("pet-leave", () => {
      const dir: 1 | -1 = body.current.x > window.innerWidth / 2 ? 1 : -1;
      leaveDir.current = dir;
      leavingRef.current = true;
      fetch.current = null;
      ball.current = null;
      setBallOn(false);
      press.current = null;
      capture(false);
      body.current = { ...body.current, y: 0, vy: 0 };
      setLeaving({ dir });
    });
    return () => void unlisten.then((off) => off());
  }, []);

  // Back to walking the next time it comes out.
  useEffect(() => {
    if (!leaving) return;
    const id = window.setTimeout(() => {
      body.current = startBody(window.innerWidth);
      leavingRef.current = false;
      setLeaving(null);
    }, 2500);
    return () => window.clearTimeout(id);
  }, [leaving]);

  // Real mouse input while the window has the mouse (the pointer is on the pet).
  const onDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || leaving) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    press.current = { x: e.clientX, y: e.clientY, dragging: false, grab: e.clientX - body.current.x };
  };
  const onMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const p = press.current;
    if (!p) return;
    if (!p.dragging && Math.hypot(e.clientX - p.x, e.clientY - p.y) > DRAG_START_PX) {
      if (!playOn.current) return;
      p.dragging = true;
      fetch.current = null;
      ball.current = null;
      setBallOn(false);
      chase.current = false;
      effect("flail", 60_000);
    }
    if (p.dragging) {
      const lift = window.innerHeight - e.clientY - 14;
      body.current = carry(body.current, e.clientX - p.grab, lift, window.innerWidth, LIFT_MAX);
    }
  };
  const onUp = (e: ReactPointerEvent<HTMLDivElement>) => {
    const p = press.current;
    press.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    if (!p) return;
    if (p.dragging) {
      body.current = drop(body.current);
      setFx(null);
      return;
    }
    // One click: a treat. Two quick clicks: fetch.
    if (clickTimer.current !== undefined) {
      window.clearTimeout(clickTimer.current);
      clickTimer.current = undefined;
      if (playOn.current) startFetch();
      return;
    }
    clickTimer.current = window.setTimeout(() => {
      clickTimer.current = undefined;
      treat();
    }, DOUBLE_CLICK_MS);
  };

  // The window lost the mouse mid-press (Alt+Tab, another window took it): no click, but a pet
  // that was being carried falls back down instead of hanging in the air.
  const letGo = () => {
    const p = press.current;
    press.current = null;
    if (p?.dragging) {
      body.current = drop(body.current);
      setFx(null);
    }
  };

  const kind = fx?.kind;
  const face = leaving
    ? "cheer"
    : kind === "treat" || kind === "purr"
      ? kind === "treat"
        ? "love"
        : "purr"
      : kind === "dizzy"
        ? "dizzy"
        : kind === "cheer" || kind === "wave"
          ? "cheer"
          : kind === "stretch"
            ? "sleepy"
            : kind === "flail"
              ? "nervous"
              : look.sulky
                ? "droopy"
                : look.face;

  return (
    <div className={`petwalk${leaving ? " petwalk--leaving" : ""}`} aria-hidden>
      <div ref={track} className="petwalk__track">
        {cloud && !leaving && (
          <div className={`petwalk__cloud petwalk__cloud--${cloud.side}`} key={cloud.at}>
            {cloud.text}
            <span className="petwalk__puff petwalk__puff--a" />
            <span className="petwalk__puff petwalk__puff--b" />
          </div>
        )}
        {kind === "treat" && !leaving && (
          <span className="petwalk__treat" key={fx?.at}>
            <span className="petwalk__cookie">🍪</span>
            <span className="petwalk__heart petwalk__heart--a">♥</span>
            <span className="petwalk__heart petwalk__heart--b">♥</span>
            <span className="petwalk__heart petwalk__heart--c">♥</span>
          </span>
        )}
        {kind === "purr" && (
          <span className="petwalk__treat" key={fx?.at}>
            <span className="petwalk__heart petwalk__heart--a">♥</span>
            <span className="petwalk__heart petwalk__heart--c">♥</span>
            <span className="petwalk__purr">purr</span>
          </span>
        )}
        {kind === "cheer" && (
          <span className="petwalk__confetti" key={fx?.at}>
            {Array.from({ length: 12 }, (_, i) => (
              <i key={i} style={{ "--i": i } as React.CSSProperties} />
            ))}
          </span>
        )}
        {kind === "juggle" && (
          <span className="petwalk__juggle" key={fx?.at}>
            <i />
            <i />
            <i />
          </span>
        )}
        {kind === "stretch" && <span className="petwalk__yawn">{"yaaawn"}</span>}
        <div
          ref={petEl}
          className={`petwalk__pet${kind ? ` petwalk__pet--${kind}` : ""}${leaving ? " petwalk__pet--leaving" : ""}`}
          style={leaving ? ({ "--face-dir": String(leaving.dir) } as React.CSSProperties) : undefined}
          onPointerDown={onDown}
          onPointerMove={onMove}
          onPointerUp={onUp}
          onPointerCancel={letGo}
          onLostPointerCapture={letGo}
        >
          <WalkerFigure
            face={face}
            prop={leaving || ballOn ? "none" : look.prop}
            outfit={look.outfit ?? "none"}
            hat={look.hat ?? ""}
            collar={!!look.collar}
            color={color}
          />
        </div>
      </div>
      {ballOn && <span ref={ballEl} className="petwalk__ball" />}
      {friendOn && (
        <div ref={friendEl} className={`petwalk__friend${kind === "wave" ? " petwalk__friend--wave" : ""}`}>
          <WalkerFigure face="cheer" prop="none" outfit="none" hat="" collar={false} color="#9fd7ff" />
        </div>
      )}
    </div>
  );
}
