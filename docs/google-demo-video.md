# Google verification: demo video script

One continuous recording, about 4–5 minutes, in English. Upload it to YouTube as **Unlisted** and paste the
link into Google Auth Platform → Verification Center. Google checks two things: that the OAuth client ID in the
sign-in URL belongs to the project being verified, and that every requested permission is shown being used.

## Before you record (10 minutes)

1. **Start clean, so every consent screen appears.**
   - Overline → Settings → Calendar → **Disconnect** (confirm "Disconnect and remove access?").
   - Turn **off** "Add events and answer invites", "Gmail glance" and "Assignment deadlines (Google Classroom)".
   - Open https://myaccount.google.com/connections and remove **Overline** if it's listed.
2. **Have something to show for each permission.**
   - Calendar: an event later today with a Google Meet link, and one invite you haven't answered (create an event
     from another account and invite yourself).
   - Gmail: send yourself 2–3 test emails from another account, and leave them unread. Close any real, private
     mail in other tabs.
   - Classroom: at least one assignment due in the next few days (any class you're in, or make one in a test class).
3. **Clean screen.** Record on screen 2 (1920×1080). Close other windows, mute notifications (Focus / Do not
   disturb), and hide desktop icons if they show anything private.
4. **Recorder.** Use Snipping Tool's screen recording (**Win+Shift+R** → drag over all of screen 2 → Start).
   Talk as you go (mic on), or add captions later; narration is clearer for Google.
5. **Magnifier** for the client ID: **Win + Plus** zooms in, **Win + Esc** turns it off. Try it once first.

## Script (say the lines in quotes)

**0. Intro (15 s)**
- Show https://dhyey2907.github.io/ in the browser.
- "This is Overline, a Windows desktop app that shows your calendar, mail and assignments in a small bar at the
  top of the screen. Everything runs on the PC; there's no server."

**1. Sign in: Calendar (40 s)**
- Overline → Settings → Calendar → **Continue with Google**.
- The browser opens. **Before choosing an account**, click into the address bar and press **Win + Plus**: zoom
  until `client_id=…apps.googleusercontent.com` is readable, hold it for 3 seconds, then **Win + Esc**.
  "Here is the OAuth client ID."
- Choose your account. The consent screen shows **Overline** and the Calendar permission. If Google shows
  "Google hasn't verified this app", that's expected for now: **Advanced → Go to Overline**.
- Tick the permission → **Continue**. Back in Overline, the row shows **Connected**.

**2. Calendar in use (40 s)** *(calendar.readonly)*
- Click the notch → **Calendar**: today's events, the next meeting, and its **Join** button.
- "Overline reads upcoming events to show the next meeting and a one-click Join."

**3. Add events and answer invites (60 s)** *(calendar.events)*
- Settings → Calendar → turn on **Add events and answer invites** → click **Reconnect with Google**.
- Address bar: **Win + Plus** on the client ID again. Show the consent screen asking to edit calendar events
  → approve.
- Notch → Calendar → **New event** (+) → type "Overline demo", pick tomorrow 10:30 → **Add to Calendar**.
- On the invite, click **Yes**.
- Open https://calendar.google.com and show the new event and your "Yes" on the invite.
- "Overline only creates an event the user typed and sends the user's own answer to an invite."

**4. Gmail (75 s)** *(gmail.modify)*
- Settings → Calendar → turn on **Gmail glance** → **Reconnect with Google** → client ID zoom → consent shows
  the Gmail permission → approve.
- Notch → **Mail**: the unread count, senders and subjects of your test emails.
- On one test email click **Archive**; on another click **Mark read**.
- Open https://mail.google.com, refresh: the archived one has left the inbox, and the other is read.
- "Overline only reads sender, subject and snippet, never message bodies. It can archive and mark as read, and it
  never sends or deletes mail. Mail data stays in memory on this PC."

**5. Classroom (45 s)** *(classroom.courses.readonly, classroom.coursework.me.readonly)*
- Settings → Calendar → turn on **Assignment deadlines (Google Classroom)** → **Reconnect with Google** → client
  ID zoom → consent → approve.
- Notch → **Day**: your upcoming assignment with its course name and due date.
- "Overline shows the user's own assignments that are due soon, with the course name. Read-only."

**6. Privacy setting (15 s)**
- Settings → Home → show **Share Google data with the planner** is **off** by default, and read its description.

**7. Disconnect (30 s)** *(openid / email were shown in Settings as the connected account)*
- Settings → Calendar → **Disconnect** → confirm.
- Open https://myaccount.google.com/connections, refresh: **Overline is gone**.
- "Disconnect removes Overline's access and deletes the sign-in from the PC." Stop recording.

## After recording

- Watch it once: the client ID must be readable in every zoom, and no private mail shows.
- YouTube → Upload → Visibility **Unlisted** → copy the link and send it to me. I'll fill in the verification
  form and show you everything before submitting.
