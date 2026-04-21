use gpui::Context;
use sqlx::{
    FromRow, SqlitePool, sqlite,
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

#[derive(sqlx::Type, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum SessionType {
    Focus,
    Break,
}

#[derive(FromRow, Debug, Clone)]
pub struct Session {
    pub id: i64,
    pub preset_id: i64,
    pub name: String,
    pub duration_in_sec: i64,
    pub color: Option<i64>,
    #[sqlx(rename = "type")]
    pub session_type: SessionType,
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
                let kind = match s.session_type {
                    SessionType::Focus => SessionKind::WORK,
                    SessionType::Break => SessionKind::BREAK,
                };
                TimerSession::new(
                    s.name.clone().into(),
                    Duration::from_secs(s.duration_in_sec.max(1) as u64),
                    kind,
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

pub struct Database {
    _pool: Option<sqlx::SqlitePool>,
    presets: Vec<Preset>,
    active_preset_id: Option<i64>,
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
            let path = PathBuf::from("bmo.db");
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
            active_preset_id: None,
        };
    }

    pub fn active_preset_id(&self) -> Option<i64> {
        self.active_preset_id
    }

    async fn load_active_preset_id(pool: &SqlitePool) -> Option<i64> {
        sqlx::query_scalar::<_, Option<i64>>(
            "SELECT active_preset_id FROM app_settings WHERE id = 1",
        )
        .fetch_one(pool)
        .await
        .ok()
        .flatten()
    }

    async fn persist_active_preset_id(pool: &SqlitePool, id: Option<i64>) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE app_settings SET active_preset_id = ? WHERE id = 1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    fn validated_active_id(presets: &[Preset], stored: Option<i64>) -> Option<i64> {
        stored.filter(|id| presets.iter().any(|p| p.id == *id))
    }

    pub fn set_active_preset_id(&mut self, id: i64) {
        self.active_preset_id = Some(id);
    }

    pub fn schedule_persist_active_preset(&self, cx: &mut Context<Self>) {
        let id = self.active_preset_id;
        let Some(pool) = self.pool() else {
            return;
        };
        cx.spawn(async move |_entity, _cx| {
            if let Err(e) = Database::persist_active_preset_id(&pool, id).await {
                eprintln!("Failed to persist active preset: {}", e);
            }
        })
        .detach();
    }

    async fn get_presets(pool: &SqlitePool) -> Result<Vec<Preset>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT
                p.id as "p_id!",
                p.name as "p_name!",
                p.description as "p_description",
                p.created_date as "p_created_date!",
                p.is_deleted as "p_is_deleted!",

                s.id as "s_id?",
                s.preset_id as "s_preset_id?",
                s.name as "s_name?",
                s.duration_in_sec as "s_duration?",
                s.color as "s_color?", -- This is already Option<i64>
                s.type as "s_type?: SessionType" -- *** THIS IS THE CHANGE ***
            FROM
                presets p
            LEFT JOIN
                session s ON p.id = s.preset_id
            WHERE
                p.is_deleted = 0
            ORDER BY
                p.id, s.id;
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut presets_map: HashMap<i64, Preset> = HashMap::new();

        for row in rows {
            // Find or create the preset
            let preset = presets_map.entry(row.p_id).or_insert_with(|| Preset {
                id: row.p_id,
                name: row.p_name.clone(),
                description: row.p_description.clone(),
                created_date: parse_sqlite_datetime(&row.p_created_date),
                is_deleted: row.p_is_deleted,
                sessions: Vec::new(),
            });

            // This is a safer way to check if a session exists
            if let Some(s_id) = row.s_id {
                // If s_id exists, we know a session row was joined.
                // We can now safely unwrap the other non-null session fields.
                preset.sessions.push(Session {
                    id: s_id,
                    preset_id: row.s_preset_id.unwrap(), // Not null in DB
                    name: row.s_name.unwrap(),           // Not null in DB
                    duration_in_sec: row.s_duration.unwrap(), // Not null in DB
                    color: row.s_color,                  // This is Option<i64>, as desired
                    session_type: row.s_type.unwrap(),   // Not null in DB (and now an enum)
                });
            }
        }

        let mut list: Vec<Preset> = presets_map.into_values().collect();
        list.sort_by_key(|p| p.id);
        Ok(list)
    }

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
        for s in template.sessions.iter() {
            let session_type = match s.kind {
                SessionKind::WORK => SessionType::Focus,
                SessionKind::BREAK => SessionType::Break,
            };
            let secs = s.duration.as_secs().max(1) as i64;
            sqlx::query(
                "INSERT INTO session (preset_id, name, duration_in_sec, color, type) VALUES (?, ?, ?, NULL, ?)",
            )
            .bind(preset_id)
            .bind(s.title.to_string())
            .bind(secs)
            .bind(session_type)
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
            if let Err(e) = Database::insert_preset_with_template_sessions(&pool, name, None).await {
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

    /// Full reload: presets from DB plus `active_preset_id` from `app_settings` (call once after connect).
    pub fn reload_from_disk(&self, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };

        cx.spawn(async move |entity, cx| {
            let entity = entity.upgrade().unwrap();
            let presets = match Database::get_presets(&pool).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to load presets: {}", e);
                    return;
                }
            };
            let stored = Database::load_active_preset_id(&pool).await;
            let validated = Database::validated_active_id(&presets, stored);
            let need_clear = stored.is_some() && validated.is_none();
            entity
                .update(cx, |this, cx| {
                    this.presets = presets;
                    this.active_preset_id = validated;
                    if need_clear {
                        this.schedule_persist_active_preset(cx);
                    }
                })
                .unwrap();
        })
        .detach();
    }

    /// Refresh preset rows only; keeps in-memory `active_preset_id` unless it no longer exists.
    pub fn update_preset_list(&self, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };

        cx.spawn(async move |entity, cx| {
            let entity = entity.upgrade().unwrap();
            let presets = match Database::get_presets(&pool).await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to load presets: {}", e);
                    return;
                }
            };
            entity
                .update(cx, |this, cx| {
                    this.presets = presets.clone();
                    let next = Database::validated_active_id(&presets, this.active_preset_id);
                    let cleared_stale =
                        this.active_preset_id != next && this.active_preset_id.is_some();
                    this.active_preset_id = next;
                    if cleared_stale {
                        this.schedule_persist_active_preset(cx);
                    }
                })
                .unwrap();
        })
        .detach();
    }
}
