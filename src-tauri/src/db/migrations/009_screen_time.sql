-- Minutes of active PC use per local hour (Home dashboard's screen time).
CREATE TABLE screen_time (
    day TEXT NOT NULL,
    hour INTEGER NOT NULL,
    seconds INTEGER NOT NULL,
    PRIMARY KEY (day, hour)
);
