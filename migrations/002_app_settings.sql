CREATE TABLE IF NOT EXISTS app_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_preset_id INTEGER
);

INSERT OR IGNORE INTO app_settings (id, active_preset_id) VALUES (1, NULL);
