-- Routines from habits: the first time each app came to the front each day.
CREATE TABLE IF NOT EXISTS app_starts (
    day TEXT NOT NULL,
    exe TEXT NOT NULL,
    path TEXT NOT NULL,
    minute INTEGER NOT NULL,
    PRIMARY KEY (day, exe)
);
