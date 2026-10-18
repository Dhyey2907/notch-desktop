-- Notch history: the cards shown in the last two days (titles only for private modules).
CREATE TABLE IF NOT EXISTS activity_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at INTEGER NOT NULL,
    module TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT,
    tone TEXT
);
CREATE INDEX IF NOT EXISTS activity_log_at ON activity_log (at);
