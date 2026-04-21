ALTER TABLE session ADD COLUMN order_index INTEGER NOT NULL DEFAULT 0;

UPDATE session
SET order_index = (
    SELECT COUNT(*)
    FROM session AS s2
    WHERE s2.preset_id = session.preset_id
      AND s2.id < session.id
);

CREATE INDEX IF NOT EXISTS idx_session_preset_order
    ON session (preset_id, order_index);
