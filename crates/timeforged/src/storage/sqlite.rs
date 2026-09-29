use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use super::summary::SummaryAccumulator;
use timeforged_core::error::AppError;
use timeforged_core::models::{
    ApiKey, Event, HourlyActivity, ReportRequest, Session, Summary, User,
};

pub async fn init_db(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::raw_sql(include_str!("migrations/001_init.sql"))
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    sqlx::raw_sql(include_str!("migrations/002_public_profile.sql"))
        .execute(pool)
        .await
        .ok(); // ignore if column already exists

    // Нормализация времени и уникальный индекс. В отличие от предыдущих,
    // ошибку здесь не глотаем: если миграция не прошла, индекса нет, и
    // повторы снова начнут копиться молча.
    sqlx::raw_sql(include_str!("migrations/003_normalize_timestamps.sql"))
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

// --- Users ---

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    display_name: Option<&str>,
) -> Result<User, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let id_str = id.to_string();
    let now_str = now.to_rfc3339();

    sqlx::query("INSERT INTO users (id, username, display_name, created_at) VALUES (?, ?, ?, ?)")
        .bind(&id_str)
        .bind(username)
        .bind(display_name)
        .bind(&now_str)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(User {
        id,
        username: username.to_string(),
        display_name: display_name.map(String::from),
        public_profile: false,
        created_at: now,
    })
}

pub async fn count_users(pool: &SqlitePool) -> Result<i64, AppError> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(row.get::<i64, _>("cnt"))
}

fn parse_user_row(row: &sqlx::sqlite::SqliteRow) -> Result<User, AppError> {
    let id_str: String = row.get("id");
    let id = Uuid::parse_str(&id_str).map_err(|e| AppError::Database(e.to_string()))?;
    let created_str: String = row.get("created_at");
    let created_at = DateTime::parse_from_rfc3339(&created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| AppError::Database(e.to_string()))?;

    let public_profile: i32 = row.try_get("public_profile").unwrap_or(0);

    Ok(User {
        id,
        username: row.get("username"),
        display_name: row.get("display_name"),
        public_profile: public_profile != 0,
        created_at,
    })
}

pub async fn get_first_user(pool: &SqlitePool) -> Result<Option<User>, AppError> {
    let row = sqlx::query("SELECT id, username, display_name, created_at FROM users ORDER BY created_at ASC LIMIT 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    row.map(|r| parse_user_row(&r)).transpose()
}

pub async fn get_user_by_username(pool: &SqlitePool, username: &str) -> Result<Option<User>, AppError> {
    let row = sqlx::query("SELECT id, username, display_name, public_profile, created_at FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    row.map(|r| parse_user_row(&r)).transpose()
}

pub async fn set_public_profile(pool: &SqlitePool, user_id: Uuid, public: bool) -> Result<(), AppError> {
    sqlx::query("UPDATE users SET public_profile = ? WHERE id = ?")
        .bind(public as i32)
        .bind(user_id.to_string())
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

// --- API Keys ---

pub async fn create_api_key(
    pool: &SqlitePool,
    user_id: Uuid,
    key_hash: &str,
    label: &str,
) -> Result<ApiKey, AppError> {
    let id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO api_keys (id, user_id, key_hash, label, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(key_hash)
    .bind(label)
    .bind(now.to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(ApiKey {
        id,
        user_id,
        key_hash: key_hash.to_string(),
        label: label.to_string(),
        created_at: now,
        last_used_at: None,
    })
}

pub async fn find_user_by_api_key_hash(
    pool: &SqlitePool,
    key_hash: &str,
) -> Result<Option<User>, AppError> {
    let row = sqlx::query(
        "SELECT u.id, u.username, u.display_name, u.created_at
         FROM users u JOIN api_keys ak ON u.id = ak.user_id
         WHERE ak.key_hash = ?",
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    match row {
        Some(row) => {
            // Update last_used_at
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE key_hash = ?")
                .bind(Utc::now().to_rfc3339())
                .bind(key_hash)
                .execute(pool)
                .await;
            Ok(Some(parse_user_row(&row)?))
        }
        None => Ok(None),
    }
}

pub async fn list_api_keys(
    pool: &SqlitePool,
    user_id: Uuid,
) -> Result<Vec<ApiKey>, AppError> {
    let rows = sqlx::query(
        "SELECT id, user_id, key_hash, label, created_at, last_used_at FROM api_keys WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    rows.iter().map(parse_api_key_row).collect()
}

pub async fn delete_api_key(
    pool: &SqlitePool,
    user_id: Uuid,
    key_id: Uuid,
) -> Result<bool, AppError> {
    let result =
        sqlx::query("DELETE FROM api_keys WHERE id = ? AND user_id = ?")
            .bind(key_id.to_string())
            .bind(user_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(result.rows_affected() > 0)
}

fn parse_api_key_row(row: &sqlx::sqlite::SqliteRow) -> Result<ApiKey, AppError> {
    let id_str: String = row.get("id");
    let user_id_str: String = row.get("user_id");
    let created_str: String = row.get("created_at");
    let last_used: Option<String> = row.get("last_used_at");

    Ok(ApiKey {
        id: Uuid::parse_str(&id_str).map_err(|e| AppError::Database(e.to_string()))?,
        user_id: Uuid::parse_str(&user_id_str).map_err(|e| AppError::Database(e.to_string()))?,
        key_hash: row.get("key_hash"),
        label: row.get("label"),
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| AppError::Database(e.to_string()))?,
        last_used_at: last_used
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc)),
    })
}

// --- Events ---

/// Формат хранения меток времени: секундный UTC.
///
/// Он один на всю базу — иначе один и тот же момент попадает в неё разными
/// строками (`+00:00`, `Z`, с наносекундами), и уникальный индекс перестаёт
/// узнавать повтор.
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";

/// Вставляет событие, молча пропуская повтор.
///
/// `OR IGNORE` работает в паре с `idx_events_unique`: повторная отправка того
/// же события при синхронизации не должна быть ошибкой — она просто ничего не
/// добавляет. Для отброшенного повтора возвращается `0`: `last_insert_rowid()`
/// после проигнорированной вставки хранит идентификатор от прошлой операции и
/// выглядел бы как успешная запись.
pub async fn insert_event(pool: &SqlitePool, event: &Event) -> Result<i64, AppError> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO events (user_id, timestamp, event_type, entity, project, language, branch, activity, machine, metadata)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event.user_id.to_string())
    .bind(event.timestamp.format(TIMESTAMP_FORMAT).to_string())
    .bind(event.event_type.as_str())
    .bind(&event.entity)
    .bind(&event.project)
    .bind(&event.language)
    .bind(&event.branch)
    .bind(event.activity.as_ref().map(|a| a.as_str()))
    .bind(&event.machine)
    .bind(event.metadata.as_ref().map(|m| m.to_string()))
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    if result.rows_affected() == 0 {
        return Ok(0);
    }
    Ok(result.last_insert_rowid())
}

pub async fn count_events(pool: &SqlitePool) -> Result<i64, AppError> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM events")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(row.get::<i64, _>("cnt"))
}

pub async fn list_events(
    pool: &SqlitePool,
    user_id: Uuid,
    since: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<Event>, AppError> {
    let rows = sqlx::query(
        "SELECT id, user_id, timestamp, event_type, entity, project, language, branch, activity, machine, metadata, created_at
         FROM events WHERE user_id = ? AND timestamp > ? ORDER BY timestamp ASC LIMIT ?",
    )
    .bind(user_id.to_string())
    .bind(since.to_rfc3339())
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    rows.iter().map(|row| {
        let id: i64 = row.get("id");
        let uid_str: String = row.get("user_id");
        let uid = Uuid::parse_str(&uid_str).map_err(|e| AppError::Database(e.to_string()))?;
        let ts_str: String = row.get("timestamp");
        let timestamp = DateTime::parse_from_rfc3339(&ts_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let et_str: String = row.get("event_type");
        let activity_str: Option<String> = row.get("activity");
        let metadata_str: Option<String> = row.get("metadata");
        let created_str: Option<String> = row.get("created_at");
        let created_at = created_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
        });

        Ok(Event {
            id: Some(id),
            user_id: uid,
            timestamp,
            event_type: timeforged_core::models::EventType::from_str_lossy(&et_str),
            entity: row.get("entity"),
            project: row.get("project"),
            language: row.get("language"),
            branch: row.get("branch"),
            activity: activity_str.map(|a| timeforged_core::models::ActivityType::from_str_lossy(&a)),
            machine: row.get("machine"),
            metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            created_at,
        })
    }).collect()
}

// --- Reports ---

pub async fn get_summary(
    pool: &SqlitePool,
    user_id: Uuid,
    req: &ReportRequest,
    idle_timeout: u64,
) -> Result<Summary, AppError> {
    let from = req.from.unwrap_or_else(|| {
        Utc::now() - chrono::Duration::days(7)
    });
    let to = req.to.unwrap_or_else(Utc::now);

    let user_id_str = user_id.to_string();
    let from_str = from.to_rfc3339();
    let to_str = to.to_rfc3339();

    // One streamed scan instead of four window-function queries, each of which re-sorted
    // the whole range (6.5s on 182k events). Global timestamp order gives every partition
    // the same relative order LAG consumed, so the accumulator reproduces the old sums;
    // memory stays O(partitions), never O(rows) -- the daemon runs under a 50MB cap.
    let mut query = String::from(
        "SELECT timestamp, project, language FROM events
         WHERE user_id = ? AND timestamp >= ? AND timestamp <= ?",
    );
    if req.project.is_some() {
        query.push_str(" AND project = ?");
    }
    query.push_str(" ORDER BY timestamp");

    let mut q = sqlx::query(&query)
        .bind(&user_id_str)
        .bind(&from_str)
        .bind(&to_str);
    if let Some(p) = req.project.as_deref() {
        q = q.bind(p);
    }

    let mut acc = SummaryAccumulator::new(idle_timeout);
    let mut rows = q.fetch(pool);
    while let Some(row) = rows.next().await {
        let row = row.map_err(|e| AppError::Database(e.to_string()))?;
        let ts_str: String = row.get("timestamp");
        let ts = DateTime::parse_from_rfc3339(&ts_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| AppError::Database(e.to_string()))?;
        let project: Option<String> = row.get("project");
        let language: Option<String> = row.get("language");
        acc.push(ts, project.as_deref(), language.as_deref());
    }

    Ok(acc.finish(from, to))
}

pub async fn get_sessions(
    pool: &SqlitePool,
    user_id: Uuid,
    req: &ReportRequest,
    idle_timeout: u64,
) -> Result<Vec<Session>, AppError> {
    let from = req.from.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
    let to = req.to.unwrap_or_else(Utc::now);

    // Gaps and the running session counter are both partitioned by project: without this,
    // events from unrelated projects that happen to land within idle_timeout of each other
    // bridge into a single fabricated cross-project "session" (see
    // timeforged-inflated-hours-2026-09-13 -- the reported 16h40m/9631-event session was
    // actually every project's events that day, merged and mislabeled).
    let mut query = String::from(
        "WITH ordered AS (
            SELECT timestamp, project,
                   LAG(timestamp) OVER (PARTITION BY project ORDER BY timestamp) as prev_ts
            FROM events
            WHERE user_id = ? AND timestamp >= ? AND timestamp <= ?",
    );
    if req.project.is_some() {
        query.push_str(" AND project = ?");
    }
    query.push_str(
        "),
        gaps AS (
            SELECT timestamp, project, prev_ts,
                   CASE WHEN prev_ts IS NULL OR (julianday(timestamp) - julianday(prev_ts)) * 86400 >= ?
                        THEN 1 ELSE 0 END as new_session
            FROM ordered
        ),
        sessions AS (
            SELECT timestamp, project,
                   SUM(new_session) OVER (PARTITION BY project ORDER BY timestamp) as session_id
            FROM gaps
        )
        SELECT MIN(timestamp) as start_ts,
               MAX(timestamp) as end_ts,
               CAST((julianday(MAX(timestamp)) - julianday(MIN(timestamp))) * 86400 AS REAL) as duration,
               project,
               COUNT(*) as event_count
        FROM sessions
        GROUP BY project, session_id
        ORDER BY start_ts",
    );

    let mut q = sqlx::query(&query)
        .bind(user_id.to_string())
        .bind(from.to_rfc3339())
        .bind(to.to_rfc3339());
    if let Some(ref p) = req.project {
        q = q.bind(p);
    }
    q = q.bind(idle_timeout as f64);

    let rows = q.fetch_all(pool).await.map_err(|e| AppError::Database(e.to_string()))?;

    rows.iter()
        .map(|r| {
            let start_str: String = r.get("start_ts");
            let end_str: String = r.get("end_ts");
            Ok(Session {
                start: DateTime::parse_from_rfc3339(&start_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| AppError::Database(e.to_string()))?,
                end: DateTime::parse_from_rfc3339(&end_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| AppError::Database(e.to_string()))?,
                duration_seconds: r.get("duration"),
                project: r.get("project"),
                event_count: r.get("event_count"),
            })
        })
        .collect()
}

pub async fn get_hourly_activity(
    pool: &SqlitePool,
    user_id: Uuid,
    req: &ReportRequest,
    idle_timeout: u64,
) -> Result<Vec<HourlyActivity>, AppError> {
    let from = req.from.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
    let to = req.to.unwrap_or_else(Utc::now);

    // Partition by project for the same reason as compute_total_seconds/get_sessions above:
    // an unpartitioned LAG lets unrelated projects bridge each other's idle gaps.
    let mut query = String::from(
        "WITH ordered AS (
            SELECT CAST(strftime('%H', timestamp) AS INTEGER) as hour,
                   timestamp,
                   LAG(timestamp) OVER (PARTITION BY project ORDER BY timestamp) as prev_ts
            FROM events
            WHERE user_id = ? AND timestamp >= ? AND timestamp <= ?",
    );
    if req.project.is_some() {
        query.push_str(" AND project = ?");
    }
    query.push_str(
        ")
        SELECT hour,
               CAST(COALESCE(SUM(
                   CASE
                       WHEN prev_ts IS NULL THEN 0.0
                       WHEN (julianday(timestamp) - julianday(prev_ts)) * 86400 < ?
                       THEN (julianday(timestamp) - julianday(prev_ts)) * 86400
                       ELSE 0.0
                   END
               ), 0.0) AS REAL) as total,
               COUNT(*) as event_count
        FROM ordered
        GROUP BY hour
        ORDER BY hour",
    );

    let mut q = sqlx::query(&query)
        .bind(user_id.to_string())
        .bind(from.to_rfc3339())
        .bind(to.to_rfc3339());
    if let Some(ref p) = req.project {
        q = q.bind(p);
    }
    q = q.bind(idle_timeout as f64);

    let rows = q.fetch_all(pool).await.map_err(|e| AppError::Database(e.to_string()))?;

    Ok(rows
        .iter()
        .map(|r| HourlyActivity {
            hour: r.get::<i32, _>("hour") as u8,
            total_seconds: r.get("total"),
            event_count: r.get("event_count"),
        })
        .collect())
}
