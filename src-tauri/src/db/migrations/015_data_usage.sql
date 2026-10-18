-- Data used per day (bytes), for the data usage meter.
CREATE TABLE IF NOT EXISTS data_usage (
    day TEXT PRIMARY KEY,
    down INTEGER NOT NULL DEFAULT 0,
    up INTEGER NOT NULL DEFAULT 0
);
