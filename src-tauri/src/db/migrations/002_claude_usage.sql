-- Claude Code usage, indexed from local transcripts (~/.claude/projects/**/*.jsonl).
-- Only counts and metadata are stored — never prompt or response text.

-- One row per assistant API message (transcripts repeat a message once per content block).
CREATE TABLE claude_messages (
    message_id     TEXT PRIMARY KEY,
    session_id     TEXT NOT NULL,
    ts             INTEGER NOT NULL,   -- unix seconds
    day            TEXT NOT NULL,      -- local YYYY-MM-DD at index time
    hour           INTEGER NOT NULL,   -- local hour 0-23
    model          TEXT NOT NULL,
    input_tokens   INTEGER NOT NULL,
    output_tokens  INTEGER NOT NULL,
    cache_read     INTEGER NOT NULL,
    cache_creation INTEGER NOT NULL
);
CREATE INDEX claude_messages_ts ON claude_messages (ts);

-- One row per prompt the user typed (tool results and meta records are excluded).
CREATE TABLE claude_prompts (
    uuid       TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    ts         INTEGER NOT NULL,
    day        TEXT NOT NULL,
    hour       INTEGER NOT NULL
);
CREATE INDEX claude_prompts_ts ON claude_prompts (ts);

-- Session → working directory, for "most active project" and live activity labels.
CREATE TABLE claude_sessions (
    session_id TEXT PRIMARY KEY,
    cwd        TEXT NOT NULL,
    first_ts   INTEGER NOT NULL,
    last_ts    INTEGER NOT NULL
);

-- How far each transcript has been read, so indexing is incremental.
CREATE TABLE claude_files (
    path   TEXT PRIMARY KEY,
    offset INTEGER NOT NULL,
    size   INTEGER NOT NULL
);
