use gpui::Context;
use sqlx::{
    FromRow, Row, SqlitePool, sqlite,
    types::chrono::{NaiveDate, NaiveDateTime},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::session::{SessionKind, TimerPreset};

fn unix_epoch_naive() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1970, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .expect("epoch")
}

/// SQLite `CURRENT_TIMESTAMP` uses `YYYY-MM-DD HH:MM:SS`; chrono's `NaiveDateTime` `FromStr` expects a `T` separator.
fn parse_sqlite_datetime(raw: &str) -> NaiveDateTime {
    let s = raw.trim();
    if s.is_empty() {
        return unix_epoch_naive();
    }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M"))
        .or_else(|_| s.parse())
        .unwrap_or_else(|_| {
            eprintln!("bmo: invalid created_date {raw:?}, using Unix epoch");
            unix_epoch_naive()
        })
}

/// Resolve the path at which `bmo.db` should live.
/// Respects `BMO_DB` env var; otherwise uses the platform app-data dir
/// (e.g. `~/Library/Application Support/Bmo/bmo.db` on macOS).
pub fn resolve_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("BMO_DB") {
        if !p.trim().is_empty() {
            let path = PathBuf::from(p);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            return path;
        }
    }
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("Bmo");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("bmo.db")
}

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum SessionType {
    Focus,
    Break,
}

impl SessionType {
    pub fn as_str(self) -> &'static str {
        match self {
            SessionType::Focus => "focus",
            SessionType::Break => "break",
        }
    }

    pub fn to_kind(self) -> SessionKind {
        match self {
            SessionType::Focus => SessionKind::WORK,
            SessionType::Break => SessionKind::BREAK,
        }
    }

    pub fn from_kind(kind: &SessionKind) -> Self {
        match kind {
            SessionKind::WORK => SessionType::Focus,
            SessionKind::BREAK => SessionType::Break,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: i64,
    pub preset_id: i64,
    pub name: String,
    pub duration_in_sec: i64,
    pub color: Option<i64>,
    pub session_type: SessionType,
    pub order_index: i64,
}

#[derive(Debug, Clone)]
pub struct Preset {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_date: NaiveDateTime,
    pub is_deleted: i64,
    pub sessions: Vec<Session>,
}

impl Preset {
    pub fn to_timer_preset(&self) -> Option<TimerPreset> {
        use crate::session::Session as TimerSession;
        use std::time::Duration;

        if self.sessions.is_empty() {
            return None;
        }
        let sessions: Vec<TimerSession> = self
            .sessions
            .iter()
            .map(|s| {
                TimerSession::new(
                    s.name.clone().into(),
                    Duration::from_secs(s.duration_in_sec.max(1) as u64),
                    s.session_type.to_kind(),
                )
            })
            .collect();
        Some(TimerPreset {
            title: self.name.clone().into(),
            source_id: Some(self.id),
            sessions,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AppPrefs {
    pub active_preset_id: Option<i64>,
    pub default_preset_id: Option<i64>,
    pub auto_advance: bool,
    pub notifications_enabled: bool,
    pub sounds_enabled: bool,
    pub theme: String,
}

impl Default for AppPrefs {
    fn default() -> Self {
        Self {
            active_preset_id: None,
            default_preset_id: None,
            auto_advance: true,
            notifications_enabled: true,
            sounds_enabled: true,
            theme: "dark".to_string(),
        }
    }
}

pub struct Database {
    _pool: Option<sqlx::SqlitePool>,
    presets: Vec<Preset>,
    prefs: AppPrefs,
}

impl Database {
    pub async fn create_db_pool(path: impl AsRef<Path>) -> Result<sqlx::SqlitePool, sqlx::Error> {
        let options = sqlite::SqliteConnectOptions::new()
            .filename(path)
            .optimize_on_close(true, None)
            .synchronous(sqlite::SqliteSynchronous::Normal)
            .journal_mode(sqlite::SqliteJournalMode::Wal)
            .create_if_missing(true)
            .statement_cache_capacity(0);

        let pool = sqlx::SqlitePool::connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        return Ok(pool);
    }

    fn pool(&self) -> Option<sqlx::SqlitePool> {
        self._pool.clone()
    }

    pub fn init(&self, cx: &mut Context<Self>) {
        cx.spawn(async |entity, cx| {
            let path = resolve_db_path();
            let pool = match Database::create_db_pool(path).await {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("Error creating connection pool: {}", e);
                    None
                }
            };
            entity
                .update(cx, |this, cx| {
                    this._pool = pool;
                    if this._pool.is_some() {
                        this.reload_from_disk(cx);
                    }
                })
                .unwrap();
        })
        .detach();
    }

    pub fn new() -> Self {
        return Self {
            _pool: None,
            presets: vec![],
            prefs: AppPrefs::default(),
        };
    }

    pub fn prefs(&self) -> &AppPrefs {
        &self.prefs
    }

    pub fn active_preset_id(&self) -> Option<i64> {
        self.prefs.active_preset_id
    }

    // ------------------------------------------------------------------
    // App-settings persistence
    // ------------------------------------------------------------------

    async fn load_prefs(pool: &SqlitePool) -> Result<AppPrefs, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT active_preset_id,
                   default_preset_id,
                   auto_advance,
                   notifications_enabled,
                   sounds_enabled,
                   theme
            FROM app_settings
            WHERE id = 1
            "#,
        )
        .fetch_one(pool)
        .await?;

        Ok(AppPrefs {
            active_preset_id: row.try_get("active_preset_id").ok(),
            default_preset_id: row.try_get("default_preset_id").ok(),
            auto_advance: row.try_get::<i64, _>("auto_advance").unwrap_or(1) != 0,
            notifications_enabled: row.try_get::<i64, _>("notifications_enabled").unwrap_or(1) != 0,
            sounds_enabled: row.try_get::<i64, _>("sounds_enabled").unwrap_or(1) != 0,
            theme: row.try_get("theme").unwrap_or_else(|_| "dark".to_string()),
        })
    }

    async fn persist_prefs(pool: &SqlitePool, prefs: &AppPrefs) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE app_settings
            SET active_preset_id       = ?,
                default_preset_id      = ?,
                auto_advance           = ?,
                notifications_enabled  = ?,
                sounds_enabled         = ?,
                theme                  = ?
            WHERE id = 1
            "#,
        )
        .bind(prefs.active_preset_id)
        .bind(prefs.default_preset_id)
        .bind(prefs.auto_advance as i64)
        .bind(prefs.notifications_enabled as i64)
        .bind(prefs.sounds_enabled as i64)
        .bind(&prefs.theme)
        .execute(pool)
        .await?;
        Ok(())
    }

    fn validated_active_id(presets: &[Preset], stored: Option<i64>) -> Option<i64> {
        stored.filter(|id| presets.iter().any(|p| p.id == *id))
    }

    pub fn set_active_preset_id(&mut self, id: i64, cx: &mut Context<Self>) {
        self.prefs.active_preset_id = Some(id);
        self.schedule_persist_prefs(cx);
        cx.notify();
    }

    pub fn clear_active_preset_id(&mut self, cx: &mut Context<Self>) {
        self.prefs.active_preset_id = None;
        self.schedule_persist_prefs(cx);
        cx.notify();
    }

    pub fn set_default_preset_id(&mut self, id: Option<i64>, cx: &mut Context<Self>) {
        self.prefs.default_preset_id = id;
        self.schedule_persist_prefs(cx);
        cx.notify();
    }

    pub fn set_auto_advance(&mut self, v: bool, cx: &mut Context<Self>) {
        self.prefs.auto_advance = v;
        self.schedule_persist_prefs(cx);
        cx.notify();
    }

    pub fn set_notifications_enabled(&mut self, v: bool, cx: &mut Context<Self>) {
        self.prefs.notifications_enabled = v;
        self.schedule_persist_prefs(cx);
        cx.notify();
    }

    pub fn set_sounds_enabled(&mut self, v: bool, cx: &mut Context<Self>) {
        self.prefs.sounds_enabled = v;
        self.schedule_persist_prefs(cx);
        cx.notify();
    }

    pub fn set_theme(&mut self, theme: String, cx: &mut Context<Self>) {
        self.prefs.theme = theme;
        self.schedule_persist_prefs(cx);
        cx.notify();
    }

    pub fn schedule_persist_active_preset(&self, cx: &mut Context<Self>) {
        self.schedule_persist_prefs(cx);
    }

    pub fn schedule_persist_prefs(&self, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };
        let prefs = self.prefs.clone();
        cx.spawn(async move |_entity, _cx| {
            if let Err(e) = Database::persist_prefs(&pool, &prefs).await {
                eprintln!("Failed to persist prefs: {}", e);
            }
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // Preset loading
    // ------------------------------------------------------------------

    async fn get_presets(pool: &SqlitePool) -> Result<Vec<Preset>, sqlx::Error> {
        #[derive(FromRow)]
        struct Row {
            p_id: i64,
            p_name: String,
            p_description: Option<String>,
            p_created_date: String,
            p_is_deleted: i64,
            s_id: Option<i64>,
            s_preset_id: Option<i64>,
            s_name: Option<String>,
            s_duration: Option<i64>,
            s_color: Option<i64>,
            s_type: Option<SessionType>,
            s_order: Option<i64>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT
                p.id              AS p_id,
                p.name            AS p_name,
                p.description     AS p_description,
                p.created_date    AS p_created_date,
                p.is_deleted      AS p_is_deleted,

                s.id              AS s_id,
                s.preset_id       AS s_preset_id,
                s.name            AS s_name,
                s.duration_in_sec AS s_duration,
                s.color           AS s_color,
                s.type            AS s_type,
                s.order_index     AS s_order
            FROM presets p
            LEFT JOIN session s ON p.id = s.preset_id
            WHERE p.is_deleted = 0
            ORDER BY p.id, s.order_index, s.id
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut presets_map: HashMap<i64, Preset> = HashMap::new();
        let mut order: Vec<i64> = Vec::new();

        for row in rows {
            if !presets_map.contains_key(&row.p_id) {
                order.push(row.p_id);
            }
            let preset = presets_map.entry(row.p_id).or_insert_with(|| Preset {
                id: row.p_id,
                name: row.p_name.clone(),
                description: row.p_description.clone(),
                created_date: parse_sqlite_datetime(&row.p_created_date),
                is_deleted: row.p_is_deleted,
                sessions: Vec::new(),
            });

            if let Some(s_id) = row.s_id {
                preset.sessions.push(Session {
                    id: s_id,
                    preset_id: row.s_preset_id.unwrap_or(row.p_id),
                    name: row.s_name.unwrap_or_default(),
                    duration_in_sec: row.s_duration.unwrap_or(60),
                    color: row.s_color,
                    session_type: row.s_type.unwrap_or(SessionType::Focus),
                    order_index: row.s_order.unwrap_or(0),
                });
            }
        }

        let mut list: Vec<Preset> = order
            .into_iter()
            .map(|id| presets_map.remove(&id).unwrap())
            .collect();
        list.sort_by_key(|p| p.id);
        Ok(list)
    }

    // ------------------------------------------------------------------
    // Preset CRUD
    // ------------------------------------------------------------------

    async fn insert_preset_with_template_sessions(
        pool: &SqlitePool,
        name: String,
        description: Option<String>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        sqlx::query("INSERT INTO presets (name, description) VALUES (?, ?)")
            .bind(&name)
            .bind(&description)
            .execute(&mut *tx)
            .await?;

        let preset_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(&mut *tx)
            .await?;

        let template = TimerPreset::default();
        for (idx, s) in template.sessions.iter().enumerate() {
            let session_type = SessionType::from_kind(&s.kind);
            let secs = s.duration.as_secs().max(1) as i64;
            sqlx::query(
                "INSERT INTO session (preset_id, name, duration_in_sec, color, type, order_index) VALUES (?, ?, ?, NULL, ?, ?)",
            )
            .bind(preset_id)
            .bind(s.title.to_string())
            .bind(secs)
            .bind(session_type)
            .bind(idx as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub fn presets(&self) -> &[Preset] {
        &self.presets
    }

    pub fn create_preset(&self, name: String, cx: &mut Context<Self>) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let Some(pool) = self.pool() else {
            return;
        };

        cx.spawn(async move |entity, cx| {
            if let Err(e) = Database::insert_preset_with_template_sessions(&pool, name, None).await
            {
                eprintln!("Failed to create preset: {}", e);
                return;
            }
            let Some(entity) = entity.upgrade() else {
                return;
            };
            let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
        })
        .detach();
    }

    pub fn rename_preset(&self, id: i64, name: String, cx: &mut Context<Self>) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let Some(pool) = self.pool() else {
            return;
        };
        cx.spawn(async move |entity, cx| {
            if let Err(e) = sqlx::query("UPDATE presets SET name = ? WHERE id = ?")
                .bind(&name)
                .bind(id)
                .execute(&pool)
                .await
            {
                eprintln!("Failed to rename preset: {}", e);
                return;
            }
            if let Some(entity) = entity.upgrade() {
                let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
            }
        })
        .detach();
    }

    pub fn soft_delete_preset(&self, id: i64, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };
        cx.spawn(async move |entity, cx| {
            if let Err(e) = sqlx::query("UPDATE presets SET is_deleted = 1 WHERE id = ?")
                .bind(id)
                .execute(&pool)
                .await
            {
                eprintln!("Failed to delete preset: {}", e);
                return;
            }
            if let Some(entity) = entity.upgrade() {
                let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
            }
        })
        .detach();
    }

    pub fn duplicate_preset(&self, id: i64, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };
        cx.spawn(async move |entity, cx| {
            if let Err(e) = Database::duplicate_preset_tx(&pool, id).await {
                eprintln!("Failed to duplicate preset: {}", e);
                return;
            }
            if let Some(entity) = entity.upgrade() {
                let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
            }
        })
        .detach();
    }

    async fn duplicate_preset_tx(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        let row = sqlx::query("SELECT name, description FROM presets WHERE id = ?")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let name: String = row.try_get("name")?;
        let description: Option<String> = row.try_get("description").ok();
        let new_name = format!("{} (copy)", name);

        sqlx::query("INSERT INTO presets (name, description) VALUES (?, ?)")
            .bind(&new_name)
            .bind(&description)
            .execute(&mut *tx)
            .await?;

        let new_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO session (preset_id, name, duration_in_sec, color, type, order_index)
             SELECT ?, name, duration_in_sec, color, type, order_index
             FROM session WHERE preset_id = ?
             ORDER BY order_index, id",
        )
        .bind(new_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Session CRUD
    // ------------------------------------------------------------------

    pub fn add_session(
        &self,
        preset_id: i64,
        name: String,
        duration_in_sec: i64,
        kind: SessionType,
        cx: &mut Context<Self>,
    ) {
        let Some(pool) = self.pool() else {
            return;
        };
        let name = if name.trim().is_empty() {
            "New session".to_string()
        } else {
            name.trim().to_string()
        };
        let duration = duration_in_sec.max(1);
        cx.spawn(async move |entity, cx| {
            let next_order: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(order_index) + 1, 0) FROM session WHERE preset_id = ?",
            )
            .bind(preset_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
            if let Err(e) = sqlx::query(
                "INSERT INTO session (preset_id, name, duration_in_sec, color, type, order_index) VALUES (?, ?, ?, NULL, ?, ?)",
            )
            .bind(preset_id)
            .bind(&name)
            .bind(duration)
            .bind(kind)
            .bind(next_order)
            .execute(&pool)
            .await
            {
                eprintln!("Failed to add session: {}", e);
                return;
            }
            if let Some(entity) = entity.upgrade() {
                let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
            }
        })
        .detach();
    }

    pub fn update_session(
        &self,
        id: i64,
        name: String,
        duration_in_sec: i64,
        kind: SessionType,
        cx: &mut Context<Self>,
    ) {
        let Some(pool) = self.pool() else {
            return;
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let duration = duration_in_sec.max(1);
        cx.spawn(async move |entity, cx| {
            if let Err(e) = sqlx::query(
                "UPDATE session SET name = ?, duration_in_sec = ?, type = ? WHERE id = ?",
            )
            .bind(&name)
            .bind(duration)
            .bind(kind)
            .bind(id)
            .execute(&pool)
            .await
            {
                eprintln!("Failed to update session: {}", e);
                return;
            }
            if let Some(entity) = entity.upgrade() {
                let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
            }
        })
        .detach();
    }

    pub fn delete_session(&self, id: i64, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };
        cx.spawn(async move |entity, cx| {
            if let Err(e) = sqlx::query("DELETE FROM session WHERE id = ?")
                .bind(id)
                .execute(&pool)
                .await
            {
                eprintln!("Failed to delete session: {}", e);
                return;
            }
            if let Some(entity) = entity.upgrade() {
                let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
            }
        })
        .detach();
    }

    pub fn reorder_sessions(&self, _preset_id: i64, ordered_ids: Vec<i64>, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };
        cx.spawn(async move |entity, cx| {
            let mut tx = match pool.begin().await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to begin tx: {}", e);
                    return;
                }
            };
            for (idx, sid) in ordered_ids.iter().enumerate() {
                if let Err(e) = sqlx::query("UPDATE session SET order_index = ? WHERE id = ?")
                    .bind(idx as i64)
                    .bind(sid)
                    .execute(&mut *tx)
                    .await
                {
                    eprintln!("Failed to reorder session {}: {}", sid, e);
                    return;
                }
            }
            if let Err(e) = tx.commit().await {
                eprintln!("Failed to commit reorder: {}", e);
                return;
            }
            if let Some(entity) = entity.upgrade() {
                let _ = entity.update(cx, |this, cx| this.update_preset_list(cx));
            }
        })
        .detach();
    }

    // ------------------------------------------------------------------
    // Reload / refresh
    // ------------------------------------------------------------------

    /// Full reload: presets from DB plus `app_settings` prefs (call once after connect).
    pub fn reload_from_disk(&self, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };

        cx.spawn(async move |entity, cx| {
            let entity = match entity.upgrade() {
                Some(e) => e,
                None => return,
            };
            let presets = match Database::get_presets(&pool).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to load presets: {}", e);
                    return;
                }
            };
            let stored_prefs = match Database::load_prefs(&pool).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to load prefs: {}", e);
                    AppPrefs::default()
                }
            };
            let mut prefs = stored_prefs;
            let validated_active = Database::validated_active_id(&presets, prefs.active_preset_id);
            let validated_default =
                Database::validated_active_id(&presets, prefs.default_preset_id);
            let need_persist = validated_active != prefs.active_preset_id
                || validated_default != prefs.default_preset_id;
            prefs.active_preset_id = validated_active;
            prefs.default_preset_id = validated_default;

            // If no active preset, fall back to default preset.
            if prefs.active_preset_id.is_none() {
                prefs.active_preset_id = prefs.default_preset_id;
            }

            let _ = entity.update(cx, |this, cx| {
                this.presets = presets;
                this.prefs = prefs;
                if need_persist {
                    this.schedule_persist_prefs(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Refresh preset rows only; keeps in-memory prefs unless active preset no longer exists.
    pub fn update_preset_list(&self, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };

        cx.spawn(async move |entity, cx| {
            let entity = match entity.upgrade() {
                Some(e) => e,
                None => return,
            };
            let presets = match Database::get_presets(&pool).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to load presets: {}", e);
                    return;
                }
            };
            let _ = entity.update(cx, |this, cx| {
                let next_active =
                    Database::validated_active_id(&presets, this.prefs.active_preset_id);
                let next_default =
                    Database::validated_active_id(&presets, this.prefs.default_preset_id);
                let dirty = next_active != this.prefs.active_preset_id
                    || next_default != this.prefs.default_preset_id;
                this.presets = presets;
                this.prefs.active_preset_id = next_active;
                this.prefs.default_preset_id = next_default;
                if dirty {
                    this.schedule_persist_prefs(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }
}
