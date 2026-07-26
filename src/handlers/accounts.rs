use axum::{extract::State, Json};
use chrono::Utc;
use constant_time_eq::constant_time_eq;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use wasm_bindgen::JsValue;
use worker::{query, Env};

use crate::{
    auth::Claims,
    db,
    error::AppError,
    models::user::{KeyData, PreloginResponse, RegisterRequest, User},
    two_factor,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeMasterPasswordRequest {
    pub master_password_hash: String,
    pub new_master_password_hash: String,
    pub master_password_hint: Option<String>,
    pub user_symmetric_key: String,
    #[serde(default)]
    pub user_asymmetric_keys: Option<KeyData>,
    #[serde(default)]
    pub kdf: Option<i32>,
    #[serde(default)]
    pub kdf_iterations: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEmailRequest {
    pub master_password_hash: String,
    pub new_master_password_hash: String,
    pub new_email: String,
    pub user_symmetric_key: String,
    #[serde(default)]
    pub kdf: Option<i32>,
    #[serde(default)]
    pub kdf_iterations: Option<i32>,
}

#[worker::send]
pub async fn profile(claims: Claims, State(env): State<Arc<Env>>) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    let two_factor_enabled = two_factor::is_authenticator_enabled(&db, &claims.sub).await?;
    let user: User = query!(&db, "SELECT * FROM users WHERE id = ?1", claims.sub)
        .map_err(|_| AppError::Database)?
        .first(None)
        .await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

    Ok(Json(json!({
        "id": user.id,
        "name": user.name,
        "email": user.email,
        "emailVerified": user.email_verified,
        "premium": true,
        "premiumFromOrganization": false,
        "masterPasswordHint": user.master_password_hint,
        "culture": "en-US",
        "twoFactorEnabled": two_factor_enabled,
        "key": user.key,
        "privateKey": user.private_key,
        "securityStamp": user.security_stamp,
        "organizations": [],
        "object": "profile"
    })))
}

#[worker::send]
pub async fn revision_date(_claims: Claims) -> Result<Json<i64>, AppError> {
    Ok(Json(chrono::Utc::now().timestamp_millis()))
}

#[worker::send]
pub async fn prelogin(
    State(env): State<Arc<Env>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<PreloginResponse>, AppError> {
    let email = payload["email"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("Missing email".to_string()))?;
    let db = db::get_db(&env)?;

    let stmt = db.prepare("SELECT kdf_iterations FROM users WHERE email = ?1");
    let query = stmt.bind(&[email.into()])?;
    let kdf_iterations: Option<i32> = query
        .first(Some("kdf_iterations"))
        .await
        .map_err(|_| AppError::Database)?;

    Ok(Json(PreloginResponse {
        kdf: 0, // PBKDF2
        kdf_iterations: kdf_iterations.unwrap_or(600_000),
        kdf_memory: None,
        kdf_parallelism: None,
    }))
}

#[worker::send]
pub async fn prelogin_password(
    State(env): State<Arc<Env>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let email = payload["email"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("Missing email".to_string()))?;
    let db = db::get_db(&env)?;
    let row: Option<serde_json::Value> = db
        .prepare("SELECT kdf_type, kdf_iterations FROM users WHERE email = ?1")
        .bind(&[email.to_lowercase().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?;
    let kdf = row
        .as_ref()
        .and_then(|value| value.get("kdf_type"))
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let iterations = row
        .as_ref()
        .and_then(|value| value.get("kdf_iterations"))
        .and_then(|value| value.as_i64())
        .unwrap_or(600_000);

    Ok(Json(json!({
        "kdf": kdf,
        "kdfIterations": iterations,
        "kdfMemory": null,
        "kdfParallelism": null,
        "kdfSettings": {
            "kdfType": kdf,
            "iterations": iterations,
            "memory": null,
            "parallelism": null
        },
        "salt": null
    })))
}

#[worker::send]
pub async fn register(
    State(env): State<Arc<Env>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    let signups_allowed = env
        .secret("SIGNUPS_ALLOWED")
        .ok()
        .and_then(|secret| secret.to_string().parse::<bool>().ok())
        .unwrap_or(false);
    if !signups_allowed {
        return Err(AppError::Unauthorized("Signups are disabled".to_string()));
    }

    let requested_email = payload.email.trim().to_lowercase();
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
    let allowlist_count: Option<i64> = db
        .prepare("SELECT COUNT(1) AS count FROM registration_allowlist")
        .first(Some("count"))
        .await
        .map_err(|_| AppError::Database)?;
    if allowlist_count.unwrap_or(0) > 0 {
        let allowed: Option<i64> = db
            .prepare("SELECT enabled FROM registration_allowlist WHERE email = ?1")
            .bind(&[requested_email.clone().into()])?
            .first(Some("enabled"))
            .await
            .map_err(|_| AppError::Database)?;
        if allowed != Some(1) {
            return Err(AppError::Unauthorized(
                "Email is not allowlisted".to_string(),
            ));
        }
    }
    let now = Utc::now().to_rfc3339();
    let auth = payload.master_password_authentication.as_ref();
    let unlock = payload.master_password_unlock.as_ref();
    let master_password_hash = payload
        .master_password_hash
        .or_else(|| auth.map(|value| value.master_password_authentication_hash.clone()))
        .ok_or_else(|| AppError::BadRequest("Missing masterPasswordHash".to_string()))?;
    let user_symmetric_key = payload
        .user_symmetric_key
        .or_else(|| unlock.map(|value| value.master_key_wrapped_user_key.clone()))
        .ok_or_else(|| AppError::BadRequest("Missing user key".to_string()))?;
    let user_asymmetric_keys = payload
        .user_asymmetric_keys
        .ok_or_else(|| AppError::BadRequest("Missing userAsymmetricKeys".to_string()))?;
    let kdf = payload
        .kdf
        .or_else(|| auth.and_then(|value| value.kdf.as_ref()?.kdf_type))
        .or_else(|| unlock.and_then(|value| value.kdf.as_ref()?.kdf_type))
        .unwrap_or(0);
    let kdf_iterations = payload
        .kdf_iterations
        .or_else(|| auth.and_then(|value| value.kdf.as_ref()?.iterations))
        .or_else(|| unlock.and_then(|value| value.kdf.as_ref()?.iterations))
        .unwrap_or(600_000);
    let user = User {
        id: Uuid::new_v4().to_string(),
        name: payload.name,
        email: payload.email.to_lowercase(),
        email_verified: true,
        master_password_hash,
        master_password_hint: payload.master_password_hint,
        key: user_symmetric_key,
        private_key: user_asymmetric_keys.encrypted_private_key,
        public_key: user_asymmetric_keys.public_key,
        kdf_type: kdf,
        kdf_iterations,
        security_stamp: Uuid::new_v4().to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    let query = query!(
        &db,
        "INSERT INTO users (id, name, email, master_password_hash, key, private_key, public_key, kdf_iterations, security_stamp, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
         user.id,
         user.name,
         user.email,
         user.master_password_hash,
         user.key,
         user.private_key,
         user.public_key,
         user.kdf_iterations,
         user.security_stamp,
         user.created_at,
         user.updated_at
    ).map_err(|error|{
        AppError::Database
    })?
    .run()
    .await
    .map_err(|error|{
        AppError::Database
    })?;

    Ok(Json(json!({})))
}

#[worker::send]
pub async fn change_master_password(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<ChangeMasterPasswordRequest>,
) -> Result<Json<Value>, AppError> {
    if payload.master_password_hash.is_empty() || payload.new_master_password_hash.is_empty() {
        return Err(AppError::BadRequest(
            "Missing masterPasswordHash".to_string(),
        ));
    }
    if payload.user_symmetric_key.is_empty() {
        return Err(AppError::BadRequest("Missing userSymmetricKey".to_string()));
    }

    let db = db::get_db(&env)?;
    let user: Value = db
        .prepare("SELECT * FROM users WHERE id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let user: User = serde_json::from_value(user).map_err(|_| AppError::Internal)?;

    if !constant_time_eq(
        user.master_password_hash.as_bytes(),
        payload.master_password_hash.as_bytes(),
    ) {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    let now = Utc::now().to_rfc3339();
    let security_stamp = Uuid::new_v4().to_string();
    let master_password_hint = payload.master_password_hint.clone();
    let private_key = payload
        .user_asymmetric_keys
        .as_ref()
        .map(|k| k.encrypted_private_key.clone())
        .unwrap_or_else(|| user.private_key.clone());
    let public_key = payload
        .user_asymmetric_keys
        .as_ref()
        .map(|k| k.public_key.clone())
        .unwrap_or_else(|| user.public_key.clone());
    let kdf_type = payload.kdf.unwrap_or(user.kdf_type);
    let kdf_iterations = payload.kdf_iterations.unwrap_or(user.kdf_iterations);

    db.prepare(
        "UPDATE users SET master_password_hash = ?1, master_password_hint = ?2, key = ?3, private_key = ?4, public_key = ?5, kdf_type = ?6, kdf_iterations = ?7, security_stamp = ?8, updated_at = ?9 WHERE id = ?10",
    )
    .bind(&[
        payload.new_master_password_hash.into(),
        to_js_val(master_password_hint),
        payload.user_symmetric_key.into(),
        private_key.into(),
        public_key.into(),
        kdf_type.into(),
        kdf_iterations.into(),
        security_stamp.into(),
        now.into(),
        claims.sub.into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;

    Ok(Json(json!({})))
}

#[worker::send]
pub async fn change_email(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<ChangeEmailRequest>,
) -> Result<Json<Value>, AppError> {
    if payload.master_password_hash.is_empty() || payload.new_master_password_hash.is_empty() {
        return Err(AppError::BadRequest(
            "Missing masterPasswordHash".to_string(),
        ));
    }
    if payload.new_email.trim().is_empty() {
        return Err(AppError::BadRequest("Missing newEmail".to_string()));
    }
    if payload.user_symmetric_key.is_empty() {
        return Err(AppError::BadRequest("Missing userSymmetricKey".to_string()));
    }

    let new_email = payload.new_email.to_lowercase();

    let db = db::get_db(&env)?;
    let user: Value = db
        .prepare("SELECT * FROM users WHERE id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let user: User = serde_json::from_value(user).map_err(|_| AppError::Internal)?;

    if !constant_time_eq(
        user.master_password_hash.as_bytes(),
        payload.master_password_hash.as_bytes(),
    ) {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    let now = Utc::now().to_rfc3339();
    let security_stamp = Uuid::new_v4().to_string();
    let kdf_type = payload.kdf.unwrap_or(user.kdf_type);
    let kdf_iterations = payload.kdf_iterations.unwrap_or(user.kdf_iterations);

    db.prepare(
        "UPDATE users SET email = ?1, email_verified = ?2, master_password_hash = ?3, key = ?4, kdf_type = ?5, kdf_iterations = ?6, security_stamp = ?7, updated_at = ?8 WHERE id = ?9",
    )
    .bind(&[
        new_email.into(),
        false.into(),
        payload.new_master_password_hash.into(),
        payload.user_symmetric_key.into(),
        kdf_type.into(),
        kdf_iterations.into(),
        security_stamp.into(),
        now.into(),
        claims.sub.into(),
    ])?
    .run()
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::BadRequest("Email already in use".to_string())
        } else {
            AppError::Database
        }
    })?;

    Ok(Json(json!({})))
}

fn to_js_val<T: Into<JsValue>>(val: Option<T>) -> JsValue {
    val.map(Into::into).unwrap_or(JsValue::NULL)
}

#[worker::send]
pub async fn send_verification_email() -> Json<String> {
    Json("local-registration-token".to_string())
}

#[worker::send]
pub async fn verification_email_clicked() -> Json<Value> {
    Json(json!({}))
}
