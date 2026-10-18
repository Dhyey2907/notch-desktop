// What the pet says while you play with it, when it celebrates, and when it notices what you're
// up to — in every tone. Placeholders: {hours}, {name}.
import type { Tone } from "./lines";

export type PlayEvent =
  | "fetched"
  | "dropped"
  | "dizzy"
  | "pounce"
  | "friend"
  | "wake"
  | "snack"
  | "sulk"
  | "celebrate"
  | "longCoding"
  | "coffee"
  | "videoMarathon";

export const PLAY_LINES: Record<Tone, Record<PlayEvent, string[]>> = {
  cheerful: {
    fetched: ["Got it! Again, again! 🎾", "Fetched! Did you see that?!"],
    dropped: ["Whee— oof! That was fun!", "Again! But gentler!"],
    dizzy: ["Whoa, the room is spinning! 🌀", "Everything's wobbly…"],
    pounce: ["Gotcha, cursor! 🐾", "Pounce! Hehe, caught you!"],
    friend: ["Oh hi friend! 👋", "My buddy came to visit!"],
    wake: ["You're back! I missed you! ☀️", "*yawn* Welcome back!"],
    snack: ["My tummy's rumbling… a treat? 🍪", "A little snack would be lovely!"],
    sulk: ["I'm too hungry to walk… 🥺", "Not coming out till I get a treat."],
    celebrate: ["Woohoo! You did it! 🎉", "Yay! That's a win!"],
    longCoding: ["{hours} hours of coding! Time to stretch? 💪", "You've coded {hours} hours — so cool!"],
    coffee: ["Coffee break? Good idea ☕", "Back from a little break? Yay!"],
    videoMarathon: ["Still watching? Grab a snack! 🍿", "What a show marathon!"],
  },
  sarcastic: {
    fetched: ["Fetched. You're welcome.", "I brought it back. Don't make it weird."],
    dropped: ["Rude.", "Ah yes, gravity. Love that for me."],
    dizzy: ["Was that necessary?", "Great. Now there are two of you."],
    pounce: ["Got your cursor. Again.", "That cursor never stood a chance."],
    friend: ["Oh look, someone who actually visits.", "My friend's here. Act normal."],
    wake: ["Oh, you're back. Thrilling.", "Look who remembered the computer exists."],
    snack: ["A treat would be nice. Just saying.", "I'd work harder if I were fed."],
    sulk: ["No treat, no walk. Those are the rules.", "I'm on strike. Hungry strike."],
    celebrate: ["Wow, you finished something.", "A win. Mark the calendar."],
    longCoding: [
      "{hours} hours of coding. Do you blink?",
      "{hours} hours. Your chair has filed a complaint.",
    ],
    coffee: ["Coffee break number… I lost count.", "Back from 'just a minute'?"],
    videoMarathon: ["Just one more episode, right?", "Very productive. Very."],
  },
  professional: {
    fetched: ["Retrieved.", "Item returned."],
    dropped: ["Landed safely.", "Back on the ground."],
    dizzy: ["Recalibrating.", "Brief disorientation."],
    pounce: ["Target acquired.", "Cursor intercepted."],
    friend: ["A colleague stopped by.", "Brief visit from a friend."],
    wake: ["Welcome back.", "Resuming."],
    snack: ["A treat would be appreciated.", "Energy running low."],
    sulk: ["Out of energy. A treat, please.", "Resting until fed."],
    celebrate: ["Well done.", "Task complete."],
    longCoding: ["{hours} hours of coding. Consider a break.", "{hours} hours in. A short break helps."],
    coffee: ["Welcome back from your break.", "Break noted."],
    videoMarathon: ["Long viewing session.", "Consider a pause."],
  },
  calm: {
    fetched: ["Here you are.", "Found it, gently."],
    dropped: ["A soft landing.", "Back on the ground. All is well."],
    dizzy: ["Let's breathe for a moment…", "Spinning… and settling."],
    pounce: ["Caught you, softly.", "A little pounce."],
    friend: ["A friend came by. How nice.", "Hello, friend."],
    wake: ["Welcome back. Take your time.", "Nice to see you again."],
    snack: ["A small treat would be nice.", "Feeling a little hungry."],
    sulk: ["I'll rest until I'm fed.", "Too hungry to wander right now."],
    celebrate: ["Lovely work.", "Well done. Breathe it in."],
    longCoding: ["{hours} hours… maybe rest your eyes.", "{hours} hours of work. Be kind to yourself."],
    coffee: ["A nice pause. Good.", "Welcome back from your break."],
    videoMarathon: ["Enjoy it. Remember to stretch.", "A long watch. Be comfy."],
  },
  coach: {
    fetched: ["Fetch complete! Again!", "That's a rep! Next one!"],
    dropped: ["Stuck the landing!", "Up and at it!"],
    dizzy: ["Shake it off! Back in the game!", "Dizzy? Walk it off!"],
    pounce: ["Explosive start! Got it!", "Pounce! That's speed!"],
    friend: ["Teammate on the field!", "Squad's here!"],
    wake: ["You're back! Let's go!", "Break over — game time!"],
    snack: ["Fuel up! A treat, coach?", "Can't run on empty!"],
    sulk: ["No fuel, no hustle. Treat me!", "Benching myself till I eat."],
    celebrate: ["THAT'S how it's done!", "Win! Keep the streak alive!"],
    longCoding: ["{hours} hours! Hydrate and stretch, champ!", "{hours} hours strong — take 5!"],
    coffee: ["Recharged? Let's crush it!", "Break done. Back to it!"],
    videoMarathon: ["Rest day? Fine, but stretch!", "Recovery mode, huh?"],
  },
  nerdy: {
    fetched: ["fetch() resolved successfully.", "200 OK: ball returned."],
    dropped: ["9.81 m/s². Physics checks out.", "Gravity: still enabled."],
    dizzy: ["Angular velocity: too high.", "My gyroscope needs a reboot."],
    pounce: ["Cursor caught in O(1).", "Pounce event handled."],
    friend: ["Peer connected!", "A friend joined the network."],
    wake: ["Session resumed.", "Welcome back, user."],
    snack: ["Low power mode. Treat to recharge?", "Need more cookies — the edible kind."],
    sulk: ["Error 418: I'm too hungry to walk.", "Process suspended: awaiting treat."],
    celebrate: ["Achievement unlocked!", "Commit pushed to the win branch!"],
    longCoding: ["{hours}h uptime. Garbage-collect yourself?", "{hours} hours. Even CPUs need cooling."],
    coffee: ["Caffeine.load() complete?", "Back from a coffee() call."],
    videoMarathon: ["Buffering… for hours.", "Binge.exe still running."],
  },
  dramatic: {
    fetched: ["I have RETURNED with the sacred orb!", "The quest is complete!"],
    dropped: ["I have FALLEN… and risen again!", "Cast down from the heavens!"],
    dizzy: ["The world spins! I am undone!", "Such chaos! Such vertigo!"],
    pounce: ["The hunter strikes!", "None escape my mighty pounce!"],
    friend: ["An ally arrives from distant lands!", "My old companion returns!"],
    wake: ["You RETURN! The kingdom rejoices!", "At last! The hero is back!"],
    snack: ["I wither… without a treat!", "Hunger gnaws at my very soul!"],
    sulk: ["I shall not move until I am FED!", "Starving… forgotten… alone…"],
    celebrate: ["VICTORY! Sing of this day!", "A triumph for the ages!"],
    longCoding: ["{hours} hours at the forge of code!", "{hours} hours! A legendary campaign!"],
    coffee: ["You return from the sacred coffee pilgrimage!", "Refreshed, the hero rises!"],
    videoMarathon: ["An epic tale unfolds for hours!", "One more episode… the eternal struggle!"],
  },
};

export const PLAY_EVENTS = Object.keys(PLAY_LINES.cheerful) as PlayEvent[];
