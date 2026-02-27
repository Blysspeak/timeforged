use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use timeforged_core::api::{CreateApiKeyRequest, CreateApiKeyResponse};
use timeforged_core::error::AppError;
use timeforged_core::models::{ApiKey, User};

use crate::storage::sqlite;

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_api_key() -> String {
    format!("tf_{}", Uuid::new_v4().to_string().replace("-", ""))
}

pub async fn create_user(
    pool: &SqlitePool,
    username: &str,
    display_name: Option<&str>,
) -> Result<User, AppError> {
    sqlite::create_user(pool, username, display_name).await
}

pub async fn authenticate(pool: &SqlitePool, api_key: &str) -> Result<User, AppError> {
    let hash = hash_api_key(api_key);
    sqlite::find_user_by_api_key_hash(pool, &hash)
        .await?
        .ok_or(AppError::Unauthorized)
}

pub async fn create_api_key(
    pool: &SqlitePool,
    user_id: Uuid,
    req: CreateApiKeyRequest,
) -> Result<CreateApiKeyResponse, AppError> {
    if req.label.is_empty() {
        return Err(AppError::Validation("label cannot be empty".into()));
    }

    let raw_key = generate_api_key();
    let hash = hash_api_key(&raw_key);

    let key = sqlite::create_api_key(pool, user_id, &hash, &req.label).await?;

    Ok(CreateApiKeyResponse {
        id: key.id,
        label: key.label,
        key: raw_key,
    })
}

pub async fn list_api_keys(pool: &SqlitePool, user_id: Uuid) -> Result<Vec<ApiKey>, AppError> {
    sqlite::list_api_keys(pool, user_id).await
}

pub async fn delete_api_key(
    pool: &SqlitePool,
    user_id: Uuid,
    key_id: Uuid,
) -> Result<(), AppError> {
    if !sqlite::delete_api_key(pool, user_id, key_id).await? {
        return Err(AppError::NotFound("api key not found".into()));
    }
    Ok(())
}

pub async fn ensure_admin(pool: &SqlitePool) -> Result<Option<String>, AppError> {
    let count = sqlite::count_users(pool).await?;
    if count > 0 {
        return Ok(None);
    }

    let user = create_user(pool, "admin", Some("Admin")).await?;
    let raw_key = generate_api_key();
    let hash = hash_api_key(&raw_key);
    sqlite::create_api_key(pool, user.id, &hash, "default").await?;

    Ok(Some(raw_key))
}
