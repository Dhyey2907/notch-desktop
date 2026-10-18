-- Price watch: product pages checked every few hours against a target price.
CREATE TABLE IF NOT EXISTS prices (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    target REAL NOT NULL,
    currency TEXT NOT NULL DEFAULT '',
    price REAL,
    checked_at INTEGER NOT NULL DEFAULT 0,
    history TEXT NOT NULL DEFAULT '[]',
    error TEXT
);
