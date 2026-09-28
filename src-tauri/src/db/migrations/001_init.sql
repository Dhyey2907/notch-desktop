-- Non-secret user preferences and module configuration (secrets live in Windows Credential Manager).
CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Cached, non-sensitive display data with a freshness timestamp (architecture.md §9).
CREATE TABLE kv_cache (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    expires_at TEXT
);
