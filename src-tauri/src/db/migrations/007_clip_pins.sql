-- Clipboard clips you pinned (history itself is kept in memory only).
CREATE TABLE clip_pins (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    text TEXT,
    files TEXT,
    image TEXT,
    width INTEGER,
    height INTEGER,
    app TEXT,
    copied_at INTEGER NOT NULL,
    pinned_at INTEGER NOT NULL
);
