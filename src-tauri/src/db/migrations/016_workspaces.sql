-- Saved window layouts (workspaces), and the rolling last session.
CREATE TABLE IF NOT EXISTS workspaces (
    name TEXT PRIMARY KEY,
    data TEXT NOT NULL,
    saved_at INTEGER NOT NULL
);
