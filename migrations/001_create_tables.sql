CREATE TABLE IF NOT EXISTS presets (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL,
    description   TEXT,
    created_date  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_deleted    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS session (
    id                INTEGER PRIMARY KEY,
    preset_id         INTEGER NOT NULL,
    name              TEXT NOT NULL,
    duration_in_sec   INTEGER NOT NULL,
    color             INTEGER NOT NULL,
    type              TEXT NOT NULL CHECK(type IN ('focus', 'break')),

    FOREIGN KEY(preset_id) REFERENCES presets(id)
);
