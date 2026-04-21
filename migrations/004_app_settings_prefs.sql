ALTER TABLE app_settings ADD COLUMN auto_advance INTEGER NOT NULL DEFAULT 1;
ALTER TABLE app_settings ADD COLUMN notifications_enabled INTEGER NOT NULL DEFAULT 1;
ALTER TABLE app_settings ADD COLUMN sounds_enabled INTEGER NOT NULL DEFAULT 1;
ALTER TABLE app_settings ADD COLUMN theme TEXT NOT NULL DEFAULT 'dark';
ALTER TABLE app_settings ADD COLUMN default_preset_id INTEGER;
