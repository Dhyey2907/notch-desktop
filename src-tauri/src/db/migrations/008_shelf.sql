-- Drop shelf: files parked on the notch (references only; the files stay where they are).
CREATE TABLE shelf_items (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    added_at INTEGER NOT NULL
);
