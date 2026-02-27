use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use timeforged_core::error::AppError;
use timeforged_core::models::{
    ApiKey, CategorySummary, DaySummary, Event, HourlyActivity, ReportRequest, Session, Summary,
    User,
};

pub async fn init_db(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::raw_sql(include_str!("migrations/001_init.sql"))
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

    Ok(User {
        id,
        username: row.get("username"),
        display_name: row.get("display_name"),
        created_at,
    })
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

pub async fn insert_event(pool: &SqlitePool, event: &Event) -> Result<i64, AppError> {
    let result = sqlx::query(
        "INSERT INTO events (user_id, timestamp, event_type, entity, project, language, branch, activity, machine, metadata)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event.user_id.to_string())
    .bind(event.timestamp.to_rfc3339())
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

    Ok(result.last_insert_rowid())
}

pub async fn count_events(pool: &SqlitePool) -> Result<i64, AppError> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM events")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(row.get::<i64, _>("cnt"))
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

    // Total time via session gaps
    let total = compute_total_seconds(pool, &user_id_str, &from_str, &to_str, idle_timeout, req.project.as_deref()).await?;

    // By project
    let projects = query_category_summary(
        pool, &user_id_str, &from_str, &to_str, "project", req.project.as_deref(), idle_timeout,
    ).await?;

    // By language
    let languages = query_category_summary(
        pool, &user_id_str, &from_str, &to_str, "language", req.project.as_deref(), idle_timeout,
    ).await?;

    // By day
    let days = query_day_summary(pool, &user_id_str, &from_str, &to_str, idle_timeout, req.project.as_deref()).await?;

    Ok(Summary {
        total_seconds: total,
        from,
        to,
        projects,
        languages,
        days,
    })
}

async fn compute_total_seconds(
    pool: &SqlitePool,
    user_id: &str,
    from: &str,
    to: &str,
    idle_timeout: u64,
    project: Option<&str>,
) -> Result<f64, AppError> {
    // Use window function to compute gaps between consecutive events.
    // If gap < idle_timeout, it's active time. Otherwise, count a flat heartbeat (e.g. 2 min).
    let mut query = String::from(
        "WITH ordered AS (
            SELECT timestamp,
                   LAG(timestamp) OVER (ORDER BY timestamp) as prev_ts
            FROM events
            WHERE user_id = ? AND timestamp >= ? AND timestamp <= ?",
    );
    if project.is_some() {
        query.push_str(" AND project = ?");
    }
    query.push_str(
        ")
        SELECT CAST(COALESCE(SUM(
            CASE
                WHEN prev_ts IS NULL THEN 0.0
                WHEN (julianday(timestamp) - julianday(prev_ts)) * 86400 < ?
                THEN (julianday(timestamp) - julianday(prev_ts)) * 86400
                ELSE 0.0
            END
        ), 0.0) AS REAL) as total
        FROM ordered",
    );

    let mut q = sqlx::query(&query)
        .bind(user_id)
        .bind(from)
        .bind(to);
    if let Some(p) = project {
        q = q.bind(p);
    }
    q = q.bind(idle_timeout as f64);

    let row = q.fetch_one(pool).await.map_err(|e| AppError::Database(e.to_string()))?;
    Ok(row.get::<f64, _>("total"))
}

async fn query_category_summary(
    pool: &SqlitePool,
    user_id: &str,
    from: &str,
    to: &str,
    column: &str,
    project_filter: Option<&str>,
    idle_timeout: u64,
) -> Result<Vec<CategorySummary>, AppError> {
    // Simpler approach: count events per category, weight by avg gap
    let mut query = format!(
        "WITH ordered AS (
            SELECT {col}, timestamp,
                   LAG(timestamp) OVER (PARTITION BY {col} ORDER BY timestamp) as prev_ts
            FROM events
            WHERE user_id = ? AND timestamp >= ? AND timestamp <= ? AND {col} IS NOT NULL",
        col = column,
    );
    if project_filter.is_some() && column != "project" {
        query.push_str(" AND project = ?");
    }
    query.push_str(&format!(
        ")
        SELECT {col} as name,
               CAST(COALESCE(SUM(
                   CASE
                       WHEN prev_ts IS NULL THEN 0.0
                       WHEN (julianday(timestamp) - julianday(prev_ts)) * 86400 < ?
                       THEN (julianday(timestamp) - julianday(prev_ts)) * 86400
                       ELSE 0.0
                   END
               ), 0.0) AS REAL) as total
        FROM ordered
        GROUP BY {col}
        ORDER BY total DESC",
        col = column,
    ));

    let mut q = sqlx::query(&query)
        .bind(user_id)
        .bind(from)
        .bind(to);
    if let Some(p) = project_filter {
        if column != "project" {
            q = q.bind(p);
        }
    }
    q = q.bind(idle_timeout as f64);

    let rows = q.fetch_all(pool).await.map_err(|e| AppError::Database(e.to_string()))?;

    let grand_total: f64 = rows.iter().map(|r| r.get::<f64, _>("total")).sum();

    Ok(rows
        .iter()
        .map(|r| {
            let total: f64 = r.get("total");
            CategorySummary {
                name: r.get("name"),
                total_seconds: total,
                percent: if grand_total > 0.0 { total / grand_total * 100.0 } else { 0.0 },
            }
        })
        .collect())
}

async fn query_day_summary(
    pool: &SqlitePool,
    user_id: &str,
    from: &str,
    to: &str,
    idle_timeout: u64,
    project: Option<&str>,
) -> Result<Vec<DaySummary>, AppError> {
    let mut query = String::from(
        "WITH ordered AS (
            SELECT date(timestamp) as day, timestamp,
                   LAG(timestamp) OVER (PARTITION BY date(timestamp) ORDER BY timestamp) as prev_ts
            FROM events
            WHERE user_id = ? AND timestamp >= ? AND timestamp <= ?",
    );
    if project.is_some() {
        query.push_str(" AND project = ?");
    }
    query.push_str(
        ")
        SELECT day,
               CAST(COALESCE(SUM(
                   CASE
                       WHEN prev_ts IS NULL THEN 0.0
                       WHEN (julianday(timestamp) - julianday(prev_ts)) * 86400 < ?
                       THEN (julianday(timestamp) - julianday(prev_ts)) * 86400
                       ELSE 0.0
                   END
               ), 0.0) AS REAL) as total
        FROM ordered
        GROUP BY day
        ORDER BY day",
    );

    let mut q = sqlx::query(&query)
        .bind(user_id)
        .bind(from)
        .bind(to);
    if let Some(p) = project {
        q = q.bind(p);
    }
    q = q.bind(idle_timeout as f64);

    let rows = q.fetch_all(pool).await.map_err(|e| AppError::Database(e.to_string()))?;

    rows.iter()
        .map(|r| {
            let day_str: String = r.get("day");
            let date = chrono::NaiveDate::parse_from_str(&day_str, "%Y-%m-%d")
                .map_err(|e| AppError::Database(e.to_string()))?;
            Ok(DaySummary {
                date,
                total_seconds: r.get("total"),
            })
        })
        .collect()
}

pub async fn get_sessions(
    pool: &SqlitePool,
    user_id: Uuid,
    req: &ReportRequest,
    idle_timeout: u64,
) -> Result<Vec<Session>, AppError> {
    let from = req.from.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
    let to = req.to.unwrap_or_else(Utc::now);

    let mut query = String::from(
        "WITH ordered AS (
            SELECT timestamp, project,
                   LAG(timestamp) OVER (ORDER BY timestamp) as prev_ts
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
                   SUM(new_session) OVER (ORDER BY timestamp) as session_id
            FROM gaps
        )
        SELECT MIN(timestamp) as start_ts,
               MAX(timestamp) as end_ts,
               CAST((julianday(MAX(timestamp)) - julianday(MIN(timestamp))) * 86400 AS REAL) as duration,
               project,
               COUNT(*) as event_count
        FROM sessions
        GROUP BY session_id
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

    let mut query = String::from(
        "WITH ordered AS (
            SELECT CAST(strftime('%H', timestamp) AS INTEGER) as hour,
                   timestamp,
                   LAG(timestamp) OVER (ORDER BY timestamp) as prev_ts
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
