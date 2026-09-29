-- To-do module: tasks (optionally with a timer) and free-form notes. Local only.
CREATE TABLE todo_tasks (
    id            TEXT PRIMARY KEY,
    title         TEXT NOT NULL,
    done          INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL,          -- unix seconds
    done_at       INTEGER,
    timer_ends_at INTEGER,                   -- set while a timer runs; cleared when it fires
    timer_secs    INTEGER,                   -- length of the last timer, for display
    ring          INTEGER NOT NULL DEFAULT 0 -- ring when the timer finishes
);
CREATE INDEX todo_tasks_timer ON todo_tasks (timer_ends_at) WHERE timer_ends_at IS NOT NULL;

CREATE TABLE todo_notes (
    id         TEXT PRIMARY KEY,
    body       TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
