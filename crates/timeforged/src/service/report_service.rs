use sqlx::SqlitePool;
use uuid::Uuid;

use timeforged_core::error::AppError;
use timeforged_core::models::{HourlyActivity, ReportRequest, Session, Summary};

use crate::storage::sqlite;

pub async fn get_summary(
    pool: &SqlitePool,
    user_id: Uuid,
    req: &ReportRequest,
    idle_timeout: u64,
) -> Result<Summary, AppError> {
    sqlite::get_summary(pool, user_id, req, idle_timeout).await
}

pub async fn get_sessions(
    pool: &SqlitePool,
    user_id: Uuid,
    req: &ReportRequest,
    idle_timeout: u64,
) -> Result<Vec<Session>, AppError> {
    sqlite::get_sessions(pool, user_id, req, idle_timeout).await
}

pub async fn get_hourly_activity(
    pool: &SqlitePool,
    user_id: Uuid,
    req: &ReportRequest,
    idle_timeout: u64,
) -> Result<Vec<HourlyActivity>, AppError> {
    sqlite::get_hourly_activity(pool, user_id, req, idle_timeout).await
}
