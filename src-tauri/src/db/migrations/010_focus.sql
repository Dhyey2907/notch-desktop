-- Focus sessions: the one running now, and how many were finished each local day.
CREATE TABLE focus_current (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    started_at INTEGER NOT NULL,
    ends_at INTEGER NOT NULL
);

CREATE TABLE focus_days (
    day TEXT PRIMARY KEY,
    sessions INTEGER NOT NULL
);
