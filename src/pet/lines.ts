// What the pet says, in the tone you picked. Placeholders: {minutes}, {title}, {agent}, {percent},
// {days}, {name}. Lines are short — they show in a small card.

import { PLAY_LINES, type PlayEvent } from "./playLines";

export type Tone = "cheerful" | "sarcastic" | "professional" | "calm" | "coach" | "nerdy" | "dramatic";

/** What it says while it sits on your desktop. */
export type BuddyEvent = "hello" | "treat" | "coding" | "watching" | "chat";

export type LineEvent =
  | BuddyEvent
  | "greeting"
  | "meeting"
  | "focusDone"
  | "breakDone"
  | "agentDone"
  | "agentWaiting"
  | "batteryLow"
  | "plugged"
  | "late"
  | "rain"
  | "pat"
  | "streak"
  | PlayEvent;

export const TONES: { id: Tone; label: string; blurb: string }[] = [
  { id: "cheerful", label: "Cheerful", blurb: "Warm, upbeat, lots of encouragement" },
  { id: "sarcastic", label: "Sarcastic", blurb: "Dry humour, gentle roasting" },
  { id: "professional", label: "Professional", blurb: "Brief and to the point" },
  { id: "calm", label: "Calm", blurb: "Soft, mindful, unhurried" },
  { id: "coach", label: "Coach", blurb: "Pushes you to win the day" },
  { id: "nerdy", label: "Nerdy", blurb: "Puns, references and fun facts" },
  { id: "dramatic", label: "Dramatic", blurb: "Everything is an epic saga" },
];

type Library = Record<Tone, Record<Exclude<LineEvent, BuddyEvent | PlayEvent>, string[]>>;

const LINES: Library = {
  cheerful: {
    greeting: ["Good morning! Let's make today lovely ☀️", "Hi hi! Ready for a great day?"],
    meeting: [
      "{title} starts in {minutes} min — you've got this!",
      "Meeting in {minutes} min! Grab some water first 💧",
    ],
    focusDone: ["Focus session done! I'm so proud of you 🎉", "Look at you go! Session complete!"],
    breakDone: ["Yay, break taken! Your eyes say thanks 👀", "Stretchy stretch! Feeling better?"],
    agentDone: ["{agent} finished! Go take a look ✨", "Ding! {agent} is done!"],
    agentWaiting: ["{agent} needs your OK!", "Psst, {agent} is waiting for you!"],
    batteryLow: [
      "Battery's at {percent}% — let's find a charger! 🔌",
      "I'm getting sleepy… {percent}% battery!",
    ],
    plugged: ["Ahh, charging! Thank you 💚", "Power! I feel brand new!"],
    late: ["It's getting late… don't forget to rest 🌙", "Past bedtime! Tomorrow-you will thank you."],
    rain: ["It's raining! Perfect cozy weather ☔", "Rain outside — stay dry out there!"],
    pat: ["Hehe, that tickles!", "Aww, thank you! 💕", "More pats please!"],
    streak: ["{days} days in a row! You're amazing!", "{days}-day streak! Keep shining ⭐"],
  },
  sarcastic: {
    greeting: ["Oh look, you're up. Shocking.", "Morning. The emails missed you. Apparently."],
    meeting: [
      "{title} in {minutes} min. Try to look like you prepared.",
      "{minutes} min until a meeting that could've been an email.",
    ],
    focusDone: ["A whole focus session. Alert the press.", "Wow, you focused. I'm only slightly impressed."],
    breakDone: [
      "A break? Look at you, being a functional human.",
      "Congrats on looking away from the screen. Revolutionary.",
    ],
    agentDone: ["{agent} finished. Somebody's doing the real work.", "{agent} is done. Unlike some of us."],
    agentWaiting: ["{agent} is waiting. Patiently. Unlike me.", "{agent} needs you. Shocking, I know."],
    batteryLow: [
      "{percent}% battery. Living dangerously, I see.",
      "I'm at {percent}%. Totally fine. Not dying at all.",
    ],
    plugged: ["Oh, NOW you plug me in.", "Charging. Took you long enough."],
    late: ["It's late. The bugs will still be here tomorrow.", "Still up? Bold choice."],
    rain: ["Rain. Great excuse to never leave this desk.", "It's raining. Not that you'd go outside anyway."],
    pat: ["Okay, okay, I get it, I'm adorable.", "That's enough affection for one day.", "Fine. One more."],
    streak: ["{days} days in a row. Who even are you?", "{days}-day streak. Don't let it go to your head."],
  },
  professional: {
    greeting: ["Good morning. Your day is set up.", "Morning. Calendar and tasks are ready."],
    meeting: ["{title} begins in {minutes} minutes.", "Reminder: meeting in {minutes} minutes."],
    focusDone: ["Focus session complete.", "Session finished. Well done."],
    breakDone: ["Break recorded.", "Break complete. Back to work when ready."],
    agentDone: ["{agent} has finished its task.", "{agent}: task complete."],
    agentWaiting: ["{agent} is awaiting your approval.", "Action needed: {agent} is waiting."],
    batteryLow: ["Battery at {percent}%. Please connect power.", "Low battery: {percent}%."],
    plugged: ["Charging.", "Power connected."],
    late: ["It's late. Consider wrapping up.", "Reminder: rest improves tomorrow's output."],
    rain: ["Rain expected. Plan travel accordingly.", "Weather update: rain."],
    pat: ["Acknowledged.", "Thank you.", "Noted, with appreciation."],
    streak: ["{days}-day streak maintained.", "Consistency: {days} days."],
  },
  calm: {
    greeting: ["Good morning. Take a slow breath before you begin.", "A new day. One thing at a time."],
    meeting: [
      "{title} in {minutes} minutes. There's time to settle in.",
      "A meeting in {minutes} minutes. Breathe, then go.",
    ],
    focusDone: ["That was a good, quiet stretch of work.", "Session complete. Notice how that felt."],
    breakDone: ["Rest well taken. Your mind thanks you.", "A pause well spent."],
    agentDone: ["{agent} has finished. Look when you're ready.", "{agent} is done, whenever you are."],
    agentWaiting: ["{agent} is waiting, no rush.", "{agent} would like your input."],
    batteryLow: [
      "Battery is at {percent}%. A charger would help.",
      "{percent}% left. Time to plug in, gently.",
    ],
    plugged: ["Charging. Everything's fine.", "Power is flowing again."],
    late: ["It's late. Rest is part of the work.", "The night is quiet. Maybe it's time to stop."],
    rain: ["Listen — it's raining.", "Rain outside. A nice time to slow down."],
    pat: ["Mm, that's nice.", "Thank you. That was kind.", "Peaceful."],
    streak: ["{days} days in a row. Steady and kind to yourself.", "{days}-day streak. Small steps add up."],
  },
  coach: {
    greeting: ["Rise and grind! Let's win today! 💪", "New day, new PRs. Let's go!"],
    meeting: [
      "{title} in {minutes}. Walk in like you own it!",
      "{minutes} minutes to showtime. Prep and crush it!",
    ],
    focusDone: ["THAT'S how it's done! Next one!", "Session down! Keep the momentum!"],
    breakDone: ["Recovery is part of training. Good work!", "Break done. Now back in the game!"],
    agentDone: ["{agent} delivered! Review it and ship it!", "{agent} is done — your move, champ!"],
    agentWaiting: ["{agent} needs a call from the captain!", "Decision time — {agent} is waiting!"],
    batteryLow: [
      "{percent}%! Refuel before the next round!",
      "Battery low at {percent}%. Hydrate the laptop!",
    ],
    plugged: ["Refuelling! Back stronger!", "Charging up. Let's go!"],
    late: ["Champions sleep. Shut it down!", "Rest is a weapon. Go to bed!"],
    rain: ["Rain won't stop us. Indoor training day!", "It's raining. No excuses, we work!"],
    pat: ["Team spirit!", "High five!", "Let's GO!"],
    streak: ["{days} days straight! Unstoppable!", "{days}-day streak! Don't break the chain!"],
  },
  nerdy: {
    greeting: ["Good morning! Booting up day.exe…", "Hello, world! Coffee: initialised."],
    meeting: ["{title} in {minutes} min. Context switch incoming.", "T-minus {minutes} minutes to {title}."],
    focusDone: ["Deep work complete. +10 XP!", "Session committed. No merge conflicts!"],
    breakDone: ["20-20-20 rule applied. Eyes recompiled.", "Garbage collection done. Memory freed!"],
    agentDone: ["{agent} returned exit code 0.", "{agent}: build succeeded ✅"],
    agentWaiting: ["{agent} is blocked on user input.", "{agent} awaits your approval. Await harder."],
    batteryLow: [
      "Battery at {percent}%. Entering low-power mode…",
      "{percent}% — we're running on fumes and caffeine.",
    ],
    plugged: ["AC adapter detected. Overclocking happiness.", "Charging. Electrons go brrr."],
    late: ["It's late. Even servers have maintenance windows.", "sudo shutdown -h now (please)"],
    rain: ["Precipitation detected. H₂O incoming.", "Rain: nature's way of debugging the sky."],
    pat: ["Affection.exe has run successfully.", "+1 happiness", "Pat acknowledged: 200 OK"],
    streak: ["{days}-day streak. O(awesome).", "Streak: {days}. That's a while(true) of greatness."],
  },
  dramatic: {
    greeting: ["THE SUN RISES and so do you, hero!", "A new chapter begins… today's saga awaits!"],
    meeting: [
      "In {minutes} minutes, the council of {title} convenes!",
      "{title}… destiny calls in {minutes} minutes!",
    ],
    focusDone: ["VICTORY! The focus session is conquered!", "Bards will sing of this session!"],
    breakDone: ["The hero rests… and rises renewed!", "A brief respite on the long road!"],
    agentDone: ["{agent} has RETURNED from its quest!", "Behold! {agent} has finished!"],
    agentWaiting: [
      "{agent} kneels before you, awaiting your command!",
      "{agent} cannot continue without your blessing!",
    ],
    batteryLow: ["My strength… fades… {percent}%…", "The light grows dim… {percent}% remains!"],
    plugged: ["POWER… COURSES THROUGH ME!", "I LIVE AGAIN!"],
    late: ["The night is long and full of bugs. Sleep, hero!", "Even legends must rest!"],
    rain: ["The heavens weep!", "A storm gathers over the land!"],
    pat: ["I shall treasure this moment forever!", "Such kindness! I am overwhelmed!", "My heart!"],
    streak: ["{days} days! A legendary streak!", "{days} days unbroken! Songs will be written!"],
  },
};

const BUDDY: Record<Tone, Record<BuddyEvent, string[]>> = {
  cheerful: {
    hello: ["I'll sit right here with you! 💜", "Hi! Comfy spot — I'm staying!"],
    treat: ["Yum!! Thank you 🍪💕", "A treat?! Best day ever!", "Nom nom nom — you're the best!"],
    coding: ["Ooh, what are we building today? 💻", "Laptop's out — let's code together!"],
    watching: ["Popcorn time! 🍿 What are we watching?", "Movie buddy reporting for duty! 🍿"],
    chat: [
      "You're doing great, you know that?",
      "Don't forget to drink some water 💧",
      "Just happy to be here with you!",
    ],
  },
  sarcastic: {
    hello: ["Fine, I'll sit here. Don't make it weird.", "Oh, so now you want company."],
    treat: ["A treat. Bribery works. Continue.", "I'll allow it. Barely.", "Crumbs everywhere. Worth it."],
    coding: [
      "Coding again? Bold of you to trust those semicolons.",
      "Laptop out. Let's write bugs together.",
    ],
    watching: ["Popcorn's ready. Your productivity isn't.", "Oh, 'just one episode'. Sure."],
    chat: [
      "Still here. Still judging. Lovingly.",
      "Have you tried turning yourself off and on again?",
      "I'd help, but I'm a cartoon.",
    ],
  },
  professional: {
    hello: ["Ready when you are.", "Standing by on your desktop."],
    treat: ["Thank you. Much appreciated.", "Treat received. Morale up.", "Noted with gratitude."],
    coding: ["Coding session detected. Focus mode.", "Laptop ready. Let's ship it."],
    watching: ["Break time. Enjoy the show.", "Leisure mode on."],
    chat: ["Remember to take short breaks.", "Hydration check.", "All systems normal."],
  },
  calm: {
    hello: ["I'll rest here quietly with you.", "A soft little spot. This is nice."],
    treat: ["Mmm… thank you. That was lovely.", "A small kindness. I feel it.", "Warm and happy now."],
    coding: ["Let's code slowly and kindly.", "One line at a time. Breathe."],
    watching: ["Let's sit back and enjoy this together.", "A gentle pause with a good story."],
    chat: ["Unclench your shoulders a little.", "Take a slow breath with me.", "You're allowed to rest too."],
  },
  coach: {
    hello: ["Spot secured. Let's get to work!", "I'm in your corner. Let's go!"],
    treat: ["Fuel acquired! Let's GO!", "Protein cookie! Gains!", "Treat earned — now earn yours!"],
    coding: ["Game time. Let's crush those tickets!", "Laptop up. Eyes on the goal!"],
    watching: ["Rest is part of training. Enjoy it!", "Recovery mode — earned it!"],
    chat: [
      "Shoulders back, keep pushing!",
      "One more task. You've got this!",
      "Water. Now. Champions hydrate.",
    ],
  },
  nerdy: {
    hello: ["Desktop instance spawned. Hello, world!", "Achievement unlocked: desk buddy."],
    treat: [
      "+1 cookie. Morale buffer overflow!",
      "Treat.exe executed successfully.",
      "Yum — that's a 10/10 commit.",
    ],
    coding: ["Compiling enthusiasm… done. Let's code!", "git commit -m 'pet joins the team'"],
    watching: ["Popcorn: the official snack of plot twists.", "Initiating binge protocol… 🍿"],
    chat: [
      "Fun fact: the first computer bug was a real moth.",
      "Have you tried rubber-duck debugging? I'm a duck now.",
      "Your uptime is impressive. Take a break?",
    ],
  },
  dramatic: {
    hello: [
      "Behold! I take my throne upon your desktop!",
      "Here I shall remain… forever! (Or until you send me back.)",
    ],
    treat: [
      "A FEAST! You honour me!",
      "The finest cookie in all the land!",
      "I shall remember this kindness for eternity!",
    ],
    coding: ["The great work begins! To the keyboard!", "Let the legendary codebase rise!"],
    watching: ["The curtain rises! The saga unfolds! 🍿", "Silence! The show begins!"],
    chat: [
      "The hours pass… and still you toil, brave one!",
      "A hero needs water too!",
      "The journey is long — but you are mighty!",
    ],
  },
};

/** One line for an event, in a tone, with its placeholders filled. `pick` chooses among them. */
export function lineFor(
  tone: string,
  event: LineEvent,
  vars: Record<string, string | number> = {},
  pick = Math.random,
): string {
  const t = (tone in LINES ? tone : "cheerful") as Tone;
  const options =
    event in BUDDY[t]
      ? BUDDY[t][event as BuddyEvent]
      : event in PLAY_LINES[t]
        ? PLAY_LINES[t][event as PlayEvent]
        : LINES[t][event as Exclude<LineEvent, BuddyEvent | PlayEvent>];
  const text = options[Math.min(options.length - 1, Math.floor(pick() * options.length))];
  return text.replace(/\{(\w+)\}/g, (_, key: string) => (key in vars ? String(vars[key]) : ""));
}

/** Least time between two lines, by chattiness. */
export const SPEAK_GAP_MS: Record<string, number> = {
  quiet: 45 * 60_000,
  normal: 15 * 60_000,
  chatty: 4 * 60_000,
};

/** Whether the pet may speak now (respecting the gap since its last line). Meetings and agents waiting always may. */
export function maySpeak(event: LineEvent, chattiness: string, lastAt: number, now: number): boolean {
  if (
    event === "meeting" ||
    event === "agentWaiting" ||
    event === "pat" ||
    event === "treat" ||
    event === "hello" ||
    // Answers to you playing with it.
    event === "fetched" ||
    event === "dropped" ||
    event === "dizzy" ||
    event === "pounce" ||
    event === "wake"
  )
    return true;
  return now - lastAt >= (SPEAK_GAP_MS[chattiness] ?? SPEAK_GAP_MS.normal);
}
