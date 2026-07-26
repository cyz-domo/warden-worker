use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use worker::Env;

use crate::{auth::Claims, db, error::AppError};

fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("Invalid email".to_string()));
    }
    Ok(email)
}

pub(crate) async fn require_admin(
    claims: &Claims,
    _headers: &HeaderMap,
    env: &Arc<Env>,
) -> Result<(), AppError> {
    let configured = env
        .secret("ADMIN_EMAIL")
        .or_else(|_| env.var("ADMIN_EMAIL"))
        .map_err(|_| AppError::Internal)?
        .to_string()
        .trim()
        .to_lowercase();
    if configured.is_empty() {
        return Err(AppError::Unauthorized(
            "Administrator access required".to_string(),
        ));
    }
    let db = db::get_db(env)?;
    let current_email: Option<String> = db
        .prepare("SELECT email FROM users WHERE id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(Some("email"))
        .await
        .map_err(|_| AppError::Database)?;
    if current_email
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase()
        != configured
    {
        return Err(AppError::Unauthorized(
            "Administrator access required".to_string(),
        ));
    }
    Ok(())
}

pub async fn require_admin_page(
    claims: &Claims,
    headers: &HeaderMap,
    env: &Arc<Env>,
) -> Result<(), AppError> {
    require_admin(claims, headers, env).await
}

async fn ensure_table(db: &worker::D1Database) -> Result<(), AppError> {
    db.prepare(
        "CREATE TABLE IF NOT EXISTS registration_allowlist (
            email TEXT PRIMARY KEY NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowlistRequest {
    pub email: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AllowlistPatch {
    pub enabled: bool,
}

#[worker::send]
pub async fn summary(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_table(&db).await?;
    let users: Option<i64> = db
        .prepare("SELECT COUNT(1) AS count FROM users")
        .first(Some("count"))
        .await
        .map_err(|_| AppError::Database)?;
    let totp: Option<i64> = db
        .prepare("SELECT COUNT(1) AS count FROM two_factor_authenticator WHERE enabled = 1")
        .first(Some("count"))
        .await
        .map_err(|_| AppError::Database)?;
    let allowlist: Option<i64> = db
        .prepare("SELECT COUNT(1) AS count FROM registration_allowlist")
        .first(Some("count"))
        .await
        .map_err(|_| AppError::Database)?;
    let enabled: Option<i64> = db
        .prepare("SELECT COUNT(1) AS count FROM registration_allowlist WHERE enabled = 1")
        .first(Some("count"))
        .await
        .map_err(|_| AppError::Database)?;
    Ok(Json(
        json!({"users": users.unwrap_or(0), "totpEnabled": totp.unwrap_or(0), "allowlist": allowlist.unwrap_or(0), "allowlistEnabled": enabled.unwrap_or(0)}),
    ))
}

#[worker::send]
pub async fn users(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_table(&db).await?;
    let rows: Vec<Value> = db
        .prepare("SELECT u.id, u.name, u.email, u.created_at, u.updated_at, CASE WHEN t.enabled = 1 THEN 1 ELSE 0 END AS two_factor_enabled FROM users u LEFT JOIN two_factor_authenticator t ON t.user_id = u.id ORDER BY u.created_at DESC")
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results()
        .map_err(|_| AppError::Database)?;
    Ok(Json(json!({"data": rows})))
}

#[worker::send]
pub async fn allowlist(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_table(&db).await?;
    let rows: Vec<Value> = db.prepare("SELECT email, enabled, created_at, updated_at FROM registration_allowlist ORDER BY email").all().await.map_err(|_| AppError::Database)?.results().map_err(|_| AppError::Database)?;
    Ok(Json(json!({"data": rows})))
}

#[worker::send]
pub async fn add_allowlist(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Json(payload): Json<AllowlistRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_table(&db).await?;
    let email = normalize_email(&payload.email)?;
    let enabled = if payload.enabled.unwrap_or(true) {
        1
    } else {
        0
    };
    let now = Utc::now().to_rfc3339();
    db.prepare("INSERT INTO registration_allowlist (email, enabled, created_at, updated_at) VALUES (?1, ?2, ?3, ?3) ON CONFLICT(email) DO UPDATE SET enabled = excluded.enabled, updated_at = excluded.updated_at")
        .bind(&[email.clone().into(), enabled.into(), now.into()])?.run().await.map_err(|_| AppError::Database)?;
    Ok(Json(json!({"email": email, "enabled": enabled == 1})))
}

#[worker::send]
pub async fn patch_allowlist(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(email): Path<String>,
    Json(payload): Json<AllowlistPatch>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_table(&db).await?;
    let email = normalize_email(&email)?;
    let now = Utc::now().to_rfc3339();
    let result = db
        .prepare("UPDATE registration_allowlist SET enabled = ?1, updated_at = ?2 WHERE email = ?3")
        .bind(&[
            (if payload.enabled { 1 } else { 0 }).into(),
            now.into(),
            email.clone().into(),
        ])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    let changes = result
        .meta()
        .map_err(|_| AppError::Database)?
        .and_then(|meta| meta.changes)
        .unwrap_or(0);
    if changes == 0 {
        return Err(AppError::NotFound("Allowlist email not found".to_string()));
    }
    Ok(Json(json!({"email": email, "enabled": payload.enabled})))
}

#[worker::send]
pub async fn delete_allowlist(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(email): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_table(&db).await?;
    let email = normalize_email(&email)?;
    db.prepare("DELETE FROM registration_allowlist WHERE email = ?1")
        .bind(&[email.into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    Ok(Json(json!({})))
}
