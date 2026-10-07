-- Gmail has no snooze in its API, so Notch hides a snoozed message itself until it's due.
-- Only message ids are kept, never mail text.
CREATE TABLE mail_snoozed (
    id TEXT PRIMARY KEY,
    until TEXT NOT NULL
);

-- Read-later shelf: links saved to read, with the page's title and an optional AI summary.
CREATE TABLE read_later (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    site TEXT NOT NULL,
    words INTEGER,
    summary TEXT,
    added_at TEXT NOT NULL,
    read_at TEXT
);
