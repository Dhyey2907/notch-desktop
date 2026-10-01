-- 2FA accounts. Only names and settings live here; each secret is in Windows Credential
-- Manager under "totp/<id>".
CREATE TABLE totp_accounts (
    id TEXT PRIMARY KEY,
    issuer TEXT NOT NULL,
    label TEXT NOT NULL,
    digits INTEGER NOT NULL,
    period INTEGER NOT NULL,
    algorithm TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Text snippets: typing the trigger in any app replaces it with the text.
CREATE TABLE snippets (
    id TEXT PRIMARY KEY,
    trigger TEXT NOT NULL UNIQUE,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL
);
