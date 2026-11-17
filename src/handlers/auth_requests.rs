use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::Utc;
use constant_time_eq::constant_time_eq;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use worker::{wasm_bindgen::JsValue, Env};

use crate::auth::Claims;
use crate::db;
use crate::error::AppError;
use crate::webauthn;

const REQUEST_TTL_SECS: i64 = 15 * 60;
const LOGIN_TTL_SECS: i64 = 5 * 60;
const MAX_EMAIL_LEN: usize = 320;
const MAX_ACCESS_CODE_LEN: usize = 512;
const MAX_PUBLIC_KEY_LEN: usize = 4096;
const MAX_WRAPPED_KEY_LEN: usize = 4096;
const MAX_PASSWORD_HASH_LEN: usize = 512;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAuthRequest {
    email: String,
    device_identifier: String,
    public_key: String,
    access_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRequestResponse {
    device_identifier: String,
    key: Option<String>,
    master_password_hash: Option<String>,
    request_approved: bool,
}

#[derive(Debug, Deserialize)]
pub struct ResponseQuery {
    code: Option<String>,
}

#[derive(Debug, Clone)]
struct AuthRequestRecord {
    id: String,
    request_device_identifier: String,
    device_type: i64,
    request_ip: String,
    access_code_hash: String,
    public_key: String,
    enc_key: Option<String>,
    master_password_hash: Option<String>,
    approved: Option<bool>,
    created_at: String,
    created_at_epoch: i64,
    response_at: Option<String>,
    authentication_at: Option<String>,
}

impl AuthRequestRecord {
    fn from_row(row: &Value) -> Option<Self> {
        Some(Self {
            id: row.get("id")?.as_str()?.to_string(),
            request_device_identifier: row.get("request_device_identifier")?.as_str()?.to_string(),
            device_type: row.get("device_type").and_then(Value::as_i64).unwrap_or(14),
            request_ip: row.get("request_ip")?.as_str()?.to_string(),
            access_code_hash: row.get("access_code_hash")?.as_str()?.to_string(),
            public_key: row.get("public_key")?.as_str()?.to_string(),
            enc_key: row
                .get("enc_key")
                .and_then(Value::as_str)
                .map(str::to_string),
            master_password_hash: row
                .get("master_password_hash")
                .and_then(Value::as_str)
                .map(str::to_string),
            approved: row
                .get("approved")
                .and_then(Value::as_i64)
                .map(|value| value != 0),
            created_at: row.get("created_at")?.as_str()?.to_string(),
            created_at_epoch: row.get("created_at_epoch")?.as_i64()?,
            response_at: row
                .get("response_at")
                .and_then(Value::as_str)
                .map(str::to_string),
            authentication_at: row
                .get("authentication_at")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    fn to_json(&self, origin: &str) -> Value {
        json!({
            "id": &self.id,
            "publicKey": &self.public_key,
            "requestDeviceType": device_type_name(self.device_type),
            "requestIpAddress": &self.request_ip,
            "key": &self.enc_key,
            "masterPasswordHash": &self.master_password_hash,
            "creationDate": &self.created_at,
            "responseDate": &self.response_at,
            "requestApproved": self.approved,
            "origin": origin,
            "object": "auth-request"
        })
    }

    fn is_expired(&self, now: i64) -> bool {
        self.created_at_epoch + REQUEST_TTL_SECS <= now
    }
}

pub async fn ensure_tables(db: &worker::D1Database) -> Result<(), AppError> {
    db.prepare(
        "CREATE TABLE IF NOT EXISTS auth_requests (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            request_device_identifier TEXT NOT NULL,
            device_type INTEGER NOT NULL DEFAULT 14,
            request_ip TEXT NOT NULL,
            response_device_id TEXT,
            access_code_hash TEXT NOT NULL,
            public_key TEXT NOT NULL,
            enc_key TEXT,
            master_password_hash TEXT,
            approved INTEGER,
            created_at TEXT NOT NULL,
            created_at_epoch INTEGER NOT NULL,
            response_at TEXT,
            authentication_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
    )
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    db.prepare("CREATE INDEX IF NOT EXISTS idx_auth_requests_user ON auth_requests(user_id)")
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    db.prepare(
        "CREATE INDEX IF NOT EXISTS idx_auth_requests_created ON auth_requests(created_at_epoch)",
    )
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    Ok(())
}

pub fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("cf-connecting-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn header_i64(headers: &HeaderMap, name: &str) -> Option<i64> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn request_device_type(headers: &HeaderMap) -> i64 {
    header_i64(headers, "device-type")
        .or_else(|| header_i64(headers, "Device-Type"))
        .unwrap_or(14)
}

fn device_type_name(value: i64) -> &'static str {
    match value {
        0 | 15 => "Android",
        1 => "iOS",
        2 => "Chrome Extension",
        3 => "Firefox Extension",
        4 => "Opera Extension",
        5 => "Edge Extension",
        6 => "Windows",
        7 => "macOS",
        8 => "Linux",
        9 => "Chrome",
        10 => "Firefox",
        11 => "Opera",
        12 => "Edge",
        13 => "Internet Explorer",
        16 => "UWP",
        17 => "Safari",
        18 => "Vivaldi",
        19 => "Vivaldi Extension",
        20 => "Safari Extension",
        21 => "SDK",
        22 => "Server",
        23 => "Windows CLI",
        24 => "macOS CLI",
        25 => "Linux CLI",
        26 => "DuckDuckGo",
        _ => "Unknown Browser",
    }
}

async fn purge_expired(db: &worker::D1Database) -> Result<(), AppError> {
    let cutoff = Utc::now().timestamp() - REQUEST_TTL_SECS;
    db.prepare("DELETE FROM auth_requests WHERE created_at_epoch < ?1")
        .bind(&[(cutoff as f64).into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    Ok(())
}

async fn load_request(
    db: &worker::D1Database,
    id: &str,
    user_id: Option<&str>,
) -> Result<Option<AuthRequestRecord>, AppError> {
    let row: Option<Value> = if let Some(user_id) = user_id {
        db.prepare(
            "SELECT id, request_device_identifier, device_type, request_ip, access_code_hash,
                    public_key, enc_key, master_password_hash, approved, created_at,
                    created_at_epoch, response_at, authentication_at
             FROM auth_requests WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[id.into(), user_id.into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?
    } else {
        db.prepare(
            "SELECT id, request_device_identifier, device_type, request_ip, access_code_hash,
                    public_key, enc_key, master_password_hash, approved, created_at,
                    created_at_epoch, response_at, authentication_at
             FROM auth_requests WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?
    };
    Ok(row.as_ref().and_then(AuthRequestRecord::from_row))
}

fn origin(env: &Env, headers: &HeaderMap) -> Result<String, AppError> {
    Ok(webauthn::rp_config(env, headers)?.origin)
}

#[worker::send]
pub async fn create(
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Json(payload): Json<CreateAuthRequest>,
) -> Result<Json<Value>, AppError> {
    let ip = client_ip(&headers);
    if !crate::ratelimit::prelogin_allowed(Some(ip.as_str())) {
        return Err(AppError::TooManyRequests);
    }

    let email = payload.email.trim().to_lowercase();
    let device_identifier = payload.device_identifier.trim().to_string();
    let public_key = payload.public_key.trim().to_string();
    let access_code = payload.access_code.trim().to_string();
    if email.is_empty() || email.len() > MAX_EMAIL_LEN {
        return Err(AppError::BadRequest("Invalid email".to_string()));
    }
    if device_identifier.is_empty() {
        return Err(AppError::BadRequest(
            "Missing device identifier".to_string(),
        ));
    }
    if public_key.is_empty() || public_key.len() > MAX_PUBLIC_KEY_LEN {
        return Err(AppError::BadRequest("Invalid public key".to_string()));
    }
    if access_code.is_empty() || access_code.len() > MAX_ACCESS_CODE_LEN {
        return Err(AppError::BadRequest("Invalid access code".to_string()));
    }

    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    purge_expired(&db).await?;

    let user_id: Option<String> = db
        .prepare("SELECT id FROM users WHERE email = ?1")
        .bind(&[email.into()])?
        .first(Some("id"))
        .await
        .map_err(|_| AppError::Database)?;
    let Some(user_id) = user_id else {
        return Err(AppError::NotFound("User not found".to_string()));
    };

    // Any device may request approval. Unknown devices stay visible to the
    // approving user because the Devices API merges pending requests in as
    // temporary device entries; no push/websocket is available here.
    let device_type = request_device_type(&headers);

    db.prepare(
        "DELETE FROM auth_requests
         WHERE user_id = ?1 AND request_device_identifier = ?2 AND approved IS NULL",
    )
    .bind(&[user_id.clone().into(), device_identifier.clone().into()])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;

    let now = Utc::now();
    let record = AuthRequestRecord {
        id: Uuid::new_v4().to_string(),
        request_device_identifier: device_identifier,
        device_type,
        request_ip: ip,
        access_code_hash: webauthn::sha256_hex(&access_code),
        public_key,
        enc_key: None,
        master_password_hash: None,
        approved: None,
        created_at: now.to_rfc3339(),
        created_at_epoch: now.timestamp(),
        response_at: None,
        authentication_at: None,
    };
    db.prepare(
        "INSERT INTO auth_requests
            (id, user_id, request_device_identifier, device_type, request_ip, response_device_id,
             access_code_hash, public_key, enc_key, master_password_hash, approved, created_at,
             created_at_epoch, response_at, authentication_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, NULL, NULL, NULL, ?8, ?9, NULL, NULL)",
    )
    .bind(&[
        record.id.clone().into(),
        user_id.into(),
        record.request_device_identifier.clone().into(),
        (record.device_type as f64).into(),
        record.request_ip.clone().into(),
        record.access_code_hash.clone().into(),
        record.public_key.clone().into(),
        record.created_at.clone().into(),
        (record.created_at_epoch as f64).into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;

    Ok(Json(record.to_json(&origin(&env, &headers)?)))
}

#[worker::send]
pub async fn get(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    purge_expired(&db).await?;
    let record = load_request(&db, &id, Some(&claims.sub))
        .await?
        .ok_or_else(|| AppError::NotFound("AuthRequest doesn't exist".to_string()))?;
    Ok(Json(record.to_json(&origin(&env, &headers)?)))
}

#[worker::send]
pub async fn respond(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<AuthRequestResponse>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    purge_expired(&db).await?;

    let Some(mut record) = load_request(&db, &id, Some(&claims.sub)).await? else {
        return Err(AppError::NotFound("AuthRequest doesn't exist".to_string()));
    };
    if record.is_expired(Utc::now().timestamp()) {
        let _ = db
            .prepare("DELETE FROM auth_requests WHERE id = ?1 AND user_id = ?2")
            .bind(&[id.clone().into(), claims.sub.clone().into()])?
            .run()
            .await;
        return Err(AppError::NotFound("AuthRequest doesn't exist".to_string()));
    }
    if record.approved.is_some() {
        return Err(AppError::BadRequest(
            "AuthRequest was already handled".to_string(),
        ));
    }

    let response_device = payload.device_identifier.trim();
    if response_device.is_empty() || claims.device.as_deref() != Some(response_device) {
        return Err(AppError::Unauthorized(
            "Device verification failed".to_string(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    if payload.request_approved {
        let key = payload.key.as_deref().map(str::trim).unwrap_or_default();
        if key.is_empty() || key.len() > MAX_WRAPPED_KEY_LEN {
            return Err(AppError::BadRequest("Invalid key".to_string()));
        }
        let master_password_hash = payload
            .master_password_hash
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if master_password_hash
            .as_deref()
            .is_some_and(|value| value.len() > MAX_PASSWORD_HASH_LEN)
        {
            return Err(AppError::BadRequest(
                "Invalid master password hash".to_string(),
            ));
        }

        let result = db
            .prepare(
                "UPDATE auth_requests
                 SET approved = 1, enc_key = ?1, master_password_hash = ?2,
                     response_device_id = ?3, response_at = ?4
                 WHERE id = ?5 AND user_id = ?6 AND approved IS NULL",
            )
            .bind(&[
                key.into(),
                master_password_hash
                    .clone()
                    .map(|value| value.into())
                    .unwrap_or(JsValue::NULL),
                response_device.into(),
                now.clone().into(),
                id.clone().into(),
                claims.sub.clone().into(),
            ])?
            .run()
            .await
            .map_err(|_| AppError::Database)?;
        let changed = result
            .meta()
            .map_err(|_| AppError::Database)?
            .and_then(|meta| meta.changes)
            .unwrap_or(0);
        if changed == 0 {
            return Err(AppError::BadRequest(
                "AuthRequest was already handled".to_string(),
            ));
        }

        record.approved = Some(true);
        record.enc_key = Some(key.to_string());
        record.master_password_hash = master_password_hash;
        record.response_at = Some(now);
    } else {
        db.prepare(
            "DELETE FROM auth_requests
             WHERE id = ?1 AND user_id = ?2 AND approved IS NULL",
        )
        .bind(&[id.clone().into(), claims.sub.clone().into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    }

    Ok(Json(record.to_json(&origin(&env, &headers)?)))
}

#[worker::send]
pub async fn response(
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<ResponseQuery>,
) -> Result<Json<Value>, AppError> {
    let ip = client_ip(&headers);
    if !crate::ratelimit::prelogin_allowed(Some(ip.as_str())) {
        return Err(AppError::TooManyRequests);
    }
    let Some(code) = query
        .code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(AppError::BadRequest("Missing access code".to_string()));
    };

    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    purge_expired(&db).await?;
    let Some(record) = load_request(&db, &id, None).await? else {
        return Err(AppError::NotFound("AuthRequest doesn't exist".to_string()));
    };
    let code_hash = webauthn::sha256_hex(code);
    if record.is_expired(Utc::now().timestamp())
        || record.request_ip != ip
        || record.device_type != request_device_type(&headers)
        || !constant_time_eq(record.access_code_hash.as_bytes(), code_hash.as_bytes())
    {
        return Err(AppError::NotFound("AuthRequest doesn't exist".to_string()));
    }

    Ok(Json(record.to_json(&origin(&env, &headers)?)))
}

#[worker::send]
pub async fn list_pending(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    purge_expired(&db).await?;
    let rows = db
        .prepare(
            "SELECT id, request_device_identifier, device_type, request_ip, access_code_hash,
                    public_key, enc_key, master_password_hash, approved, created_at,
                    created_at_epoch, response_at, authentication_at
             FROM auth_requests
             WHERE user_id = ?1 AND approved IS NULL
             ORDER BY created_at_epoch DESC",
        )
        .bind(&[claims.sub.clone().into()])?
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results::<Value>()
        .map_err(|_| AppError::Database)?;
    let origin = origin(&env, &headers)?;
    let data: Vec<Value> = rows
        .iter()
        .filter_map(AuthRequestRecord::from_row)
        .map(|record| record.to_json(&origin))
        .collect();

    Ok(Json(json!({
        "data": data,
        "continuationToken": null,
        "object": "list"
    })))
}

/// Validates the access code for a pending auth request during password grant login.
pub async fn verify_login(
    db: &worker::D1Database,
    id: &str,
    user_id: &str,
    ip: &str,
    access_code: &str,
) -> Result<bool, AppError> {
    ensure_tables(db).await?;
    let Some(record) = load_request(db, id, Some(user_id)).await? else {
        return Ok(false);
    };
    let code_hash = webauthn::sha256_hex(access_code);
    Ok(!record.is_expired(Utc::now().timestamp())
        && record.approved == Some(true)
        && record.authentication_at.is_none()
        && record.request_ip == ip
        && constant_time_eq(record.access_code_hash.as_bytes(), code_hash.as_bytes()))
}

/// Marks an auth request as used. Returns false if it was already consumed.
pub async fn consume(db: &worker::D1Database, id: &str, user_id: &str) -> Result<bool, AppError> {
    let now = Utc::now().to_rfc3339();
    let result = db
        .prepare(
            "UPDATE auth_requests SET authentication_at = ?1
             WHERE id = ?2 AND user_id = ?3 AND approved = 1 AND authentication_at IS NULL
               AND created_at_epoch >= ?4",
        )
        .bind(&[
            now.into(),
            id.into(),
            user_id.into(),
            ((Utc::now().timestamp() - LOGIN_TTL_SECS) as f64).into(),
        ])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    Ok(result
        .meta()
        .map_err(|_| AppError::Database)?
        .and_then(|meta| meta.changes)
        .unwrap_or(0)
        == 1)
}

#[derive(Debug, Clone)]
pub struct PendingAuthRequest {
    pub id: String,
    pub creation_date: String,
    pub device_type: i64,
}

impl PendingAuthRequest {
    pub fn to_device_json(&self) -> Value {
        json!({
            "id": &self.id,
            "creationDate": &self.creation_date
        })
    }
}

/// Pending auth requests keyed by requesting device identifier, for the Devices view.
pub async fn pending_by_device(
    db: &worker::D1Database,
    user_id: &str,
) -> Result<HashMap<String, PendingAuthRequest>, AppError> {
    ensure_tables(db).await?;
    purge_expired(db).await?;
    let cutoff = Utc::now().timestamp() - REQUEST_TTL_SECS;
    let rows = db
        .prepare(
            "SELECT id, request_device_identifier, device_type, created_at
             FROM auth_requests
             WHERE user_id = ?1 AND approved IS NULL AND created_at_epoch >= ?2
             ORDER BY created_at_epoch DESC",
        )
        .bind(&[user_id.into(), (cutoff as f64).into()])?
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results::<Value>()
        .map_err(|_| AppError::Database)?;

    let mut pending = HashMap::new();
    for row in rows {
        let Some(identifier) = row.get("request_device_identifier").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(created_at) = row.get("created_at").and_then(Value::as_str) else {
            continue;
        };
        pending
            .entry(identifier.to_string())
            .or_insert_with(|| PendingAuthRequest {
                id: id.to_string(),
                creation_date: created_at.to_string(),
                device_type: row.get("device_type").and_then(Value::as_i64).unwrap_or(14),
            });
    }
    Ok(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_ip_prefers_cloudflare_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.1, 10.0.0.1".parse().unwrap());
        assert_eq!(client_ip(&headers), "203.0.113.1");

        headers.insert("cf-connecting-ip", "198.51.100.7".parse().unwrap());
        assert_eq!(client_ip(&headers), "198.51.100.7");
    }

    #[test]
    fn device_type_names_match_bitwarden() {
        assert_eq!(device_type_name(2), "Chrome Extension");
        assert_eq!(device_type_name(6), "Windows");
        assert_eq!(device_type_name(26), "DuckDuckGo");
        assert_eq!(device_type_name(999), "Unknown Browser");
    }
}
