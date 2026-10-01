-- Minutes spent in an editor per project per day (project names come from editor titles).
CREATE TABLE coding_minutes (
    day TEXT NOT NULL,
    project TEXT NOT NULL,
    minutes INTEGER NOT NULL,
    PRIMARY KEY (day, project)
);
