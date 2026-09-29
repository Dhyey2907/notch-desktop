-- Every permission decision made through Notch, for review in Settings → AI.
-- The command/target is stored here (and only here, never in logs) because an audit trail
-- that doesn't say what was approved is useless. Old rows are pruned.
CREATE TABLE approval_audit (
    id          TEXT PRIMARY KEY,
    ts          INTEGER NOT NULL,       -- unix seconds
    session_id  TEXT NOT NULL,
    project     TEXT NOT NULL,
    tool        TEXT NOT NULL,
    summary     TEXT NOT NULL,          -- the exact command, file path, or URL
    destructive INTEGER NOT NULL,       -- 0/1
    decision    TEXT NOT NULL,          -- allow | deny | ask
    decided_by  TEXT NOT NULL           -- you | session_rule | timeout | app_closed
);
CREATE INDEX approval_audit_ts ON approval_audit (ts);
