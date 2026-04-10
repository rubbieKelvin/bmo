use gpui::Context;
use sqlx::{FromRow, SqlitePool, sqlite, types::chrono::NaiveDateTime};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

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

pub struct Database {
    _pool: Option<sqlx::SqlitePool>,
    presets: Vec<Preset>,
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
                        this.update_preset_list(cx);
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
        };
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
                p.id;
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
                created_date: NaiveDateTime::from_str(&row.p_created_date).unwrap(),
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

        Ok(presets_map.into_values().collect())
    }

    pub fn update_preset_list(&self, cx: &mut Context<Self>) {
        let Some(pool) = self.pool() else {
            return;
        };

        cx.spawn(async move |entity, cx| {
            let entity = entity.upgrade().unwrap();
            let presets = Database::get_presets(&pool).await.unwrap();
            entity
                .update(cx, |this, _cx| {
                    this.presets = presets;
                })
                .unwrap();
        })
        .detach();
    }
}
