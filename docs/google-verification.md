# Google OAuth verification — Notch Desktop

Goal: anyone can "Continue with Google" without the *unverified app* warning, without the 100-user cap,
and without sign-ins expiring every 7 days.

Scopes requested (incrementally — calendar editing, Gmail and Classroom only when the user switches them on):

| Scope | Class | Used for |
|---|---|---|
| `openid`, `email` | basic | Show which account is connected |
| `https://www.googleapis.com/auth/calendar.readonly` | sensitive | Next meeting, reminders, one-click Join |
| `https://www.googleapis.com/auth/calendar.events` | sensitive | Opt-in: **New event** form, Quick capture events, **Yes / Maybe / No** on invites |
| `https://www.googleapis.com/auth/gmail.modify` | **restricted** | Unread count + newest senders/subjects; **Archive** and **Mark as read** buttons |
| `https://www.googleapis.com/auth/classroom.courses.readonly` | sensitive | Course names on deadlines |
| `https://www.googleapis.com/auth/classroom.coursework.me.readonly` | sensitive | Assignment titles, due dates, turned-in state |

Because of `gmail.modify` (restricted), Google may also ask for a **CASA security assessment** (yearly,
roughly $540–$4,500 through an approved lab). Notch has no server — all Google data stays on the user's PC
— and Google's restricted-scope page ties the assessment to apps that access data "from or through a
third-party server", so say so clearly in the submission; Google decides whether it applies.

---

## Step 1 — Website on a domain you own (needs your OK to publish)

The site is in `site/` (home, privacy policy, terms, logos). Google requires the homepage and privacy
policy to be on an **authorized domain that you have verified** in Google Search Console.

Recommended free hosting: a public GitHub repository named **`Dhyey2907.github.io`** containing the files
of `site/` → served at **https://dhyey2907.github.io/**.

- Homepage: `https://dhyey2907.github.io/`
- Privacy policy: `https://dhyey2907.github.io/privacy.html`
- Terms: `https://dhyey2907.github.io/terms.html`

Before publishing, replace every `CONTACT_EMAIL` in `site/*.html` with the public contact address you want
to use (it's shown on the site and must match the consent screen's developer contact).

Then verify ownership: Google Search Console → Add property → **URL prefix** `https://dhyey2907.github.io/`
→ *HTML tag* → paste the `<meta name="google-site-verification" …>` tag into `site/index.html`'s `<head>`,
publish, click Verify. (Use the same Google account that owns the Cloud project.)

## Step 2 — Google Cloud Console → Google Auth Platform

Project: the one that owns Notch's built-in OAuth client (`src-tauri/oauth-clients.json`).

**Branding**
- App name: `Notch Desktop` (must match the website and the app exactly)
- User support email: your address
- App logo: `site/google-logo-120.png` (120×120)
- Application home page: `https://dhyey2907.github.io/`
- Privacy policy: `https://dhyey2907.github.io/privacy.html`
- Terms of service: `https://dhyey2907.github.io/terms.html`
- Authorized domains: `dhyey2907.github.io`
- Developer contact: your address

**Audience**: User type **External** → **Publish app** (Testing → In production).

**Data access** → add exactly the six scopes above. Justifications to paste:

- *calendar.readonly*: "Notch Desktop is a Windows desktop app that shows the user's next Google Calendar
  event in a small bar at the top of their screen, reminds them shortly before it starts, and offers a
  one-click button to join the video meeting. It only reads upcoming events (title, time, location,
  meeting link); it never creates, edits or deletes events. Data is fetched by the app on the user's PC
  and never sent to our servers (we have none)."
- *calendar.events*: "Only when the user turns on 'Add events and answer invites' in Settings, Notch shows
  a New event form (title, day, time, length, place) and Yes / Maybe / No buttons on meeting invites. Each
  action happens only when the user clicks it: events.insert / events.quickAdd for an event the user typed,
  and events.patch of the user's own attendee responseStatus for an invite they answer. Notch never deletes
  events and never edits events beyond the user's own answer. calendar.readonly cannot create events or
  answer invites, which is why this scope is needed. Data stays on the user's PC."
- *gmail.modify*: "When the user turns on Gmail in Settings, Notch shows their unread count and the sender
  and subject of the newest unread inbox messages, and notifies them of new mail. Each message in the
  notch has 'Archive' and 'Mark as read' buttons, which remove that message's INBOX or UNREAD label via
  users.messages.modify — this is why gmail.readonly is not enough. Notch requests format=metadata only
  (From and Subject headers plus Gmail's snippet); it never reads message bodies or attachments, never
  sends, drafts or deletes mail, and never changes settings. Snooze is done by Notch itself: it keeps only
  the snoozed message's id and time on the PC and hides it until then (no label is changed). All processing happens on the user's
  computer; mail data is kept only in memory and is never transmitted to any server of ours or to any AI
  service."
- *classroom.courses.readonly*: "Used to show the name of the course next to each upcoming assignment
  deadline in the notch."
- *classroom.coursework.me.readonly*: "Used to show the user's own assignments that are due soon (title,
  due date, link) and to hide ones they've already turned in, as reminders in the notch."
- *openid / email*: "Shows the user which Google account is connected in Settings."

## Step 3 — Demo video (unlisted YouTube link)

Record the screen (Win+Alt+R, or OBS) in English, ~3–5 minutes. Google checks that the OAuth client ID in
the consent URL matches the project, and that every scope is shown in use.

1. Open Notch → Settings → Calendar. Say what the app is.
2. Click **Continue with Google**. When the browser opens, **zoom into the address bar** so the
   `client_id=…` is readable. Show the consent screen with the app name "Notch Desktop" and the Calendar
   permission. Approve.
3. Back in Notch: show the next meeting in the notch, the reminder card, and the **Join** button.
4. Settings → Calendar → turn on **Gmail**. Show the second consent screen (Gmail permission) with the
   client ID visible. Approve. Open the Mail module: unread count, senders/subjects. Click **Mark as read**
   on one message and **Archive** on another, then show in Gmail (web) that they changed.
5. Turn on **Classroom**. Show its consent screen, then the deadlines in the notch (Home/Your day).
6. Show Settings → Home → **Share Google data with the planner** (off by default) and read its text.
7. Show **Disconnect** in Settings → Calendar, and that access disappears from
   myaccount.google.com/permissions.

## Step 4 — Submit

Verification Center → **Prepare for verification** → fill in the video link and justifications →
Submit. Typical timelines: brand verification a few days; sensitive scopes ~2–6 weeks; restricted scope
review longer. Google emails the developer contact with questions — answer from the same project.

## What changed in the app for verification

- **Explicit consent before Google data reaches an AI**: the AI day planner gets today's meetings and
  Classroom deadlines only if *Settings → Home → Share Google data with the planner* is on (default off).
  Gmail data is never sent to any AI.
- **Disconnect** revokes the token at Google and deletes cached events and the saved day plan; mail and
  Classroom data are dropped from memory.
- Gmail and Classroom permissions are requested only when the user turns those features on.
