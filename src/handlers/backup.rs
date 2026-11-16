use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use worker::{D1Database, Env};

use crate::auth::Claims;
use crate::backup::{
    backup_key, decrypt_backup, encrypt_backup, encryption_ready, ensure_tables, export_payload,
    next_run_after, now_secs, parse_payload, parse_time, restore_payload, serialize_payload,
};
use crate::db;
use crate::error::AppError;
use crate::webdav::WebDavClient;

use super::admin::require_admin;

const MAX_NAME: usize = 80;
const MAX_URL: usize = 2048;
const MAX_USERNAME: usize = 200;
const MAX_PASSWORD: usize = 512;
const MAX_KEEP: i64 = 100;
const MAX_IMPORT_BYTES: usize = 24 * 1024 * 1024;

#[derive(Clone)]
struct ChannelRow {
    id: String,
    name: String,
    url: String,
    username: Option<String>,
    password_enc: Option<String>,
    enabled: bool,
    cadence: String,
    time: String,
    weekday: i64,
    utc_offset_minutes: i64,
    keep_count: i64,
    next_run_at: Option<i64>,
    last_run_at: Option<i64>,
    last_status: Option<String>,
    last_error: Option<String>,
}

fn text_field(row: &Map<String, Value>, key: &str) -> Option<String> {
    row.get(key).and_then(Value::as_str).map(str::to_string)
}

fn int_field(row: &Map<String, Value>, key: &str, fallback: i64) -> i64 {
    row.get(key)
        .and_then(Value::as_i64)
        .or_else(|| {
            row.get(key)
                .and_then(Value::as_f64)
                .map(|value| value as i64)
        })
        .unwrap_or(fallback)
}

fn bool_field(row: &Map<String, Value>, key: &str) -> bool {
    match row.get(key) {
        Some(Value::Bool(flag)) => *flag,
        Some(value) => value.as_i64().is_some_and(|number| number != 0),
        None => false,
    }
}

fn js_opt_string(value: Option<String>) -> JsValue {
    value
        .map(|text| JsValue::from_str(&text))
        .unwrap_or(JsValue::NULL)
}

fn js_opt_i64(value: Option<i64>) -> JsValue {
    value
        .map(|number| JsValue::from_f64(number as f64))
        .unwrap_or(JsValue::NULL)
}

fn js_i64(value: i64) -> JsValue {
    JsValue::from_f64(value as f64)
}

fn opt_int_field(row: &Map<String, Value>, key: &str) -> Option<i64> {
    row.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_f64().map(|number| number as i64))
    })
}

fn channel_from_value(value: Value) -> Result<ChannelRow, AppError> {
    let row = value.as_object().ok_or(AppError::Internal)?;
    Ok(ChannelRow {
        id: text_field(row, "id").ok_or(AppError::Internal)?,
        name: text_field(row, "name").ok_or(AppError::Internal)?,
        url: text_field(row, "url").ok_or(AppError::Internal)?,
        username: text_field(row, "username"),
        password_enc: text_field(row, "password_enc"),
        enabled: bool_field(row, "enabled"),
        cadence: text_field(row, "cadence").unwrap_or_else(|| "off".to_string()),
        time: text_field(row, "time").unwrap_or_else(|| "03:00".to_string()),
        weekday: int_field(row, "weekday", 1),
        utc_offset_minutes: int_field(row, "utc_offset_minutes", 480),
        keep_count: int_field(row, "keep_count", 7),
        next_run_at: opt_int_field(row, "next_run_at"),
        last_run_at: opt_int_field(row, "last_run_at"),
        last_status: text_field(row, "last_status"),
        last_error: text_field(row, "last_error"),
    })
}

fn sanitized(channel: &ChannelRow) -> Value {
    json!({
        "id": channel.id,
        "name": channel.name,
        "url": channel.url,
        "username": channel.username,
        "enabled": channel.enabled,
        "cadence": channel.cadence,
        "time": channel.time,
        "weekday": channel.weekday,
        "utcOffsetMinutes": channel.utc_offset_minutes,
        "keepCount": channel.keep_count,
        "nextRunAt": channel.next_run_at,
        "lastRunAt": channel.last_run_at,
        "lastStatus": channel.last_status,
        "lastError": channel.last_error,
        "hasPassword": channel.password_enc.is_some(),
    })
}

fn encrypt_password(key: &[u8; 32], channel_id: &str, password: &str) -> Result<String, AppError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut iv);
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: password.as_bytes(),
                aad: channel_id.as_bytes(),
            },
        )
        .map_err(|_| AppError::Crypto("Could not encrypt the WebDAV password".to_string()))?;
    Ok(format!(
        "v1:{}:{}",
        general_purpose::STANDARD.encode(iv),
        general_purpose::STANDARD.encode(encrypted)
    ))
}

fn decrypt_password(
    key: &[u8; 32],
    channel_id: &str,
    stored: Option<&str>,
) -> Result<Option<String>, AppError> {
    let Some(stored) = stored else {
        return Ok(None);
    };
    let mut parts = stored.split(':');
    if parts.next() != Some("v1") {
        return Err(AppError::Crypto(
            "The stored WebDAV password uses an unsupported format".to_string(),
        ));
    }
    let iv = parts
        .next()
        .and_then(|text| general_purpose::STANDARD.decode(text).ok())
        .filter(|bytes| bytes.len() == 12)
        .ok_or(AppError::Crypto(
            "The stored WebDAV password is damaged".to_string(),
        ))?;
    let data = parts
        .next()
        .and_then(|text| general_purpose::STANDARD.decode(text).ok())
        .ok_or(AppError::Crypto(
            "The stored WebDAV password is damaged".to_string(),
        ))?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &data,
                aad: channel_id.as_bytes(),
            },
        )
        .map_err(|_| {
            AppError::Crypto(
                "The stored WebDAV password cannot be decrypted with the current BACKUP_ENC_KEY"
                    .to_string(),
            )
        })?;
    String::from_utf8(plaintext)
        .map(Some)
        .map_err(|_| AppError::Crypto("The stored WebDAV password is not UTF-8".to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInput {
    pub name: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub enabled: Option<bool>,
    pub cadence: Option<String>,
    pub time: Option<String>,
    pub weekday: Option<i64>,
    pub utc_offset_minutes: Option<i64>,
    pub keep_count: Option<i64>,
}

struct ValidatedChannel {
    name: String,
    url: String,
    username: Option<String>,
    password: Option<String>,
    enabled: bool,
    cadence: String,
    time: String,
    weekday: i64,
    utc_offset_minutes: i64,
    keep_count: i64,
}

fn validate_channel(input: &ChannelInput) -> Result<ValidatedChannel, AppError> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return Err(AppError::BadRequest(
            "Use a channel name of 1 to 80 characters.".to_string(),
        ));
    }
    if input.url.chars().count() > MAX_URL {
        return Err(AppError::BadRequest(
            "The WebDAV URL is too long.".to_string(),
        ));
    }
    // Constructing the client validates the URL scheme, host, and shape.
    let client = WebDavClient::new(&input.url, None, None)?;
    let url = client.base_url();
    let username = input
        .username
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if username
        .as_ref()
        .is_some_and(|value| value.len() > MAX_USERNAME)
    {
        return Err(AppError::BadRequest(
            "The WebDAV username is too long.".to_string(),
        ));
    }
    let password = input
        .password
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if password.is_some() && username.is_none() {
        return Err(AppError::BadRequest(
            "Enter the WebDAV username when a password is set.".to_string(),
        ));
    }
    if password
        .as_ref()
        .is_some_and(|value| value.len() > MAX_PASSWORD)
    {
        return Err(AppError::BadRequest(
            "The WebDAV password is too long.".to_string(),
        ));
    }
    let cadence = input.cadence.clone().unwrap_or_else(|| "daily".to_string());
    if !matches!(cadence.as_str(), "off" | "daily" | "weekly") {
        return Err(AppError::BadRequest(
            "Choose an off, daily, or weekly schedule.".to_string(),
        ));
    }
    let time = input.time.clone().unwrap_or_else(|| "03:00".to_string());
    parse_time(&time)?;
    let weekday = input.weekday.unwrap_or(1);
    if !(0..=6).contains(&weekday) {
        return Err(AppError::BadRequest(
            "Choose a weekday from Sunday to Saturday.".to_string(),
        ));
    }
    let utc_offset_minutes = input.utc_offset_minutes.unwrap_or(480);
    if !(-720..=840).contains(&utc_offset_minutes) {
        return Err(AppError::BadRequest(
            "The UTC offset must be between -720 and 840 minutes.".to_string(),
        ));
    }
    let keep_count = input.keep_count.unwrap_or(7);
    if !(1..=MAX_KEEP).contains(&keep_count) {
        return Err(AppError::BadRequest(format!(
            "Keep between 1 and {MAX_KEEP} backups."
        )));
    }
    Ok(ValidatedChannel {
        name: name.to_string(),
        url,
        username,
        password,
        enabled: input.enabled.unwrap_or(true),
        cadence,
        time,
        weekday,
        utc_offset_minutes,
        keep_count,
    })
}

fn next_run(channel: &ValidatedChannel, now: i64) -> Result<Option<i64>, AppError> {
    if !channel.enabled || channel.cadence == "off" {
        return Ok(None);
    }
    next_run_after(
        now,
        &channel.cadence,
        &channel.time,
        channel.weekday,
        channel.utc_offset_minutes,
    )
    .map(Some)
}

async fn load_channel(db: &D1Database, id: &str) -> Result<ChannelRow, AppError> {
    let row: Option<Value> = db
        .prepare("SELECT * FROM backup_channels WHERE id = ?1")
        .bind(&[id.into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?;
    row.map(channel_from_value)
        .transpose()?
        .ok_or_else(|| AppError::NotFound("Backup channel not found.".to_string()))
}

async fn channels(db: &D1Database) -> Result<Vec<ChannelRow>, AppError> {
    let rows: Vec<Value> = db
        .prepare("SELECT * FROM backup_channels ORDER BY created_at")
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results()
        .map_err(|_| AppError::Database)?;
    rows.into_iter().map(channel_from_value).collect()
}

fn client_for(channel: &ChannelRow, key: &[u8; 32]) -> Result<WebDavClient, AppError> {
    let password = decrypt_password(key, &channel.id, channel.password_enc.as_deref())?;
    WebDavClient::new(
        &channel.url,
        channel.username.as_deref(),
        password.as_deref(),
    )
}

async fn release_lock(db: &D1Database, channel_id: &str) {
    if let Ok(statement) = db
        .prepare("DELETE FROM backup_locks WHERE channel_id = ?1")
        .bind(&[channel_id.into()])
    {
        let _ = statement.run().await;
    }
}

async fn record_run_start(
    db: &D1Database,
    channel_id: &str,
    trigger: &str,
    now: i64,
) -> Result<String, AppError> {
    let run_id = uuid::Uuid::new_v4().to_string();
    db.prepare(
        "INSERT INTO backup_runs (id, channel_id, trigger, status, started_at) VALUES (?1, ?2, ?3, 'running', ?4)",
    )
    .bind(&[
        run_id.clone().into(),
        channel_id.into(),
        trigger.into(),
        js_i64(now),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    Ok(run_id)
}

async fn record_run_finish(
    db: &D1Database,
    run_id: &str,
    status: &str,
    file_name: Option<&str>,
    size: Option<i64>,
    error: Option<&str>,
    now: i64,
) -> Result<(), AppError> {
    db.prepare(
        "UPDATE backup_runs SET status = ?1, file_name = ?2, size = ?3, error = ?4, finished_at = ?5 WHERE id = ?6",
    )
    .bind(&[
        status.into(),
        js_opt_string(file_name.map(str::to_string)),
        js_opt_i64(size),
        js_opt_string(error.map(str::to_string)),
        js_i64(now),
        run_id.into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    Ok(())
}

async fn update_channel_after_run(
    db: &D1Database,
    channel: &ChannelRow,
    now: i64,
    status: &str,
    error: Option<&str>,
) -> Result<(), AppError> {
    let next = if channel.enabled && channel.cadence != "off" {
        Some(next_run_after(
            now,
            &channel.cadence,
            &channel.time,
            channel.weekday,
            channel.utc_offset_minutes,
        )?)
    } else {
        None
    };
    db.prepare(
        "UPDATE backup_channels SET last_run_at = ?1, last_status = ?2, last_error = ?3, next_run_at = ?4, updated_at = ?5 WHERE id = ?6",
    )
    .bind(&[
        js_i64(now),
        status.into(),
        error
            .map(|value| JsValue::from_str(&value.chars().take(500).collect::<String>()))
            .unwrap_or(JsValue::NULL),
        js_opt_i64(next),
        Utc::now().to_rfc3339().into(),
        channel.id.clone().into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    Ok(())
}

fn backup_name(prefix: &str) -> String {
    let mut suffix = [0u8; 3];
    OsRng.fill_bytes(&mut suffix);
    format!(
        "{prefix}-{}-{}.enc",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        hex::encode(suffix)
    )
}

async fn create_snapshot(
    env: &Arc<Env>,
    client: &WebDavClient,
    prefix: &str,
) -> Result<(String, i64), AppError> {
    let key = backup_key(env)?;
    let db = db::get_db(env)?;
    let payload = export_payload(&db).await?;
    let plaintext = serialize_payload(&payload)?;
    let envelope = encrypt_backup(&key, &plaintext)?;
    let bytes = serde_json::to_vec(&envelope).map_err(|_| AppError::Internal)?;
    let name = backup_name(prefix);
    client.ensure_dir().await?;
    client.put(&name, &bytes).await?;
    Ok((name, bytes.len() as i64))
}

async fn prune(client: &WebDavClient, keep_count: i64, current: &str) -> Result<(), AppError> {
    let names = client.list().await?;
    let keep = keep_count.max(1) as usize;
    for name in names
        .iter()
        .filter(|name| name.as_str() != current)
        .skip(keep.saturating_sub(1))
    {
        if let Err(error) = client.delete(name).await {
            log::warn!("Could not delete old backup {name}: {error}");
        }
    }
    Ok(())
}

async fn run_channel(
    env: &Arc<Env>,
    channel: &ChannelRow,
    trigger: &str,
) -> Result<Value, AppError> {
    let db = db::get_db(env)?;
    ensure_tables(&db).await?;
    let now = now_secs();
    // A short lease keeps two cron invocations from uploading the same backup concurrently.
    let acquired = db
        .prepare(
            "INSERT INTO backup_locks (channel_id, locked_until) VALUES (?1, ?2)
             ON CONFLICT(channel_id) DO UPDATE SET locked_until = excluded.locked_until
             WHERE backup_locks.locked_until < ?3",
        )
        .bind(&[channel.id.clone().into(), js_i64(now + 900), js_i64(now)])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    let changes = acquired
        .meta()
        .map_err(|_| AppError::Database)?
        .and_then(|meta| meta.changes)
        .unwrap_or(0);
    if changes == 0 {
        return Err(AppError::BadRequest(
            "A backup is already running for this channel.".to_string(),
        ));
    }

    let run_id = match record_run_start(&db, &channel.id, trigger, now).await {
        Ok(run_id) => run_id,
        Err(error) => {
            release_lock(&db, &channel.id).await;
            return Err(error);
        }
    };
    let outcome = async {
        let key = backup_key(env)?;
        let client = client_for(channel, &key)?;
        let (file_name, size) = create_snapshot(env, &client, "warden").await?;
        prune(&client, channel.keep_count, &file_name).await?;
        Ok::<(String, i64), AppError>((file_name, size))
    }
    .await;

    release_lock(&db, &channel.id).await;

    match outcome {
        Ok((file_name, size)) => {
            if let Err(error) = update_channel_after_run(&db, channel, now, "success", None).await {
                log::warn!("Could not record backup channel status: {error}");
            }
            if let Err(error) = record_run_finish(
                &db,
                &run_id,
                "success",
                Some(&file_name),
                Some(size),
                None,
                now_secs(),
            )
            .await
            {
                log::warn!("Could not record backup run result: {error}");
            }
            Ok(json!({"file": file_name, "size": size}))
        }
        Err(error) => {
            let message = error.to_string();
            log::error!("Backup failed for {}: {message}", channel.id);
            if let Err(update_error) =
                update_channel_after_run(&db, channel, now, "error", Some(&message)).await
            {
                log::warn!("Could not record backup channel status: {update_error}");
            }
            if let Err(run_error) = record_run_finish(
                &db,
                &run_id,
                "error",
                None,
                None,
                Some(&message),
                now_secs(),
            )
            .await
            {
                log::warn!("Could not record backup run result: {run_error}");
            }
            Err(error)
        }
    }
}

pub async fn run_scheduled(env: &Arc<Env>) {
    let Ok(db) = db::get_db(env) else {
        return;
    };
    if ensure_tables(&db).await.is_err() {
        return;
    }
    let Ok(statement) = db
        .prepare(
            "SELECT * FROM backup_channels
             WHERE enabled = 1 AND cadence != 'off' AND next_run_at IS NOT NULL AND next_run_at <= ?1
             ORDER BY next_run_at LIMIT 2",
        )
        .bind(&[js_i64(now_secs())])
    else {
        return;
    };
    let Ok(result) = statement.all().await else {
        return;
    };
    let Ok(rows) = result.results::<Value>() else {
        return;
    };
    for row in rows {
        let Ok(channel) = channel_from_value(row) else {
            continue;
        };
        if let Err(error) = run_channel(env, &channel, "scheduled").await {
            log::warn!("Scheduled backup skipped for {}: {error}", channel.id);
        }
    }
}

#[worker::send]
pub async fn list_channels(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let items: Vec<Value> = channels(&db).await?.iter().map(sanitized).collect();
    Ok(Json(json!({
        "data": items,
        "encryptionReady": encryption_ready(&env),
    })))
}

#[worker::send]
pub async fn create_channel(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Json(input): Json<ChannelInput>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let validated = validate_channel(&input)?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let password_enc = match validated.password.as_deref() {
        Some(password) => Some(encrypt_password(&backup_key(&env)?, &id, password)?),
        None => None,
    };
    let now = now_secs();
    let next = next_run(&validated, now)?;
    db.prepare(
        "INSERT INTO backup_channels
         (id, name, type, url, username, password_enc, enabled, cadence, time, weekday, utc_offset_minutes, keep_count, next_run_at, created_at, updated_at)
         VALUES (?1, ?2, 'webdav', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
    )
    .bind(&[
        id.clone().into(),
        validated.name.into(),
        validated.url.into(),
        js_opt_string(validated.username),
        js_opt_string(password_enc),
        (if validated.enabled { 1 } else { 0 }).into(),
        validated.cadence.into(),
        validated.time.into(),
        js_i64(validated.weekday),
        js_i64(validated.utc_offset_minutes),
        js_i64(validated.keep_count),
        js_opt_i64(next),
        Utc::now().to_rfc3339().into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    let channel = load_channel(&db, &id).await?;
    Ok(Json(sanitized(&channel)))
}

#[worker::send]
pub async fn update_channel(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(id): Path<String>,
    Json(input): Json<ChannelInput>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let validated = validate_channel(&input)?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let existing = load_channel(&db, &id).await?;
    let password_enc = match validated.password.as_deref() {
        Some(password) => Some(encrypt_password(&backup_key(&env)?, &id, password)?),
        None => existing.password_enc.clone(),
    };
    let now = now_secs();
    let next = next_run(&validated, now)?;
    db.prepare(
        "UPDATE backup_channels SET name = ?1, url = ?2, username = ?3, password_enc = ?4,
         enabled = ?5, cadence = ?6, time = ?7, weekday = ?8, utc_offset_minutes = ?9,
         keep_count = ?10, next_run_at = ?11, updated_at = ?12 WHERE id = ?13",
    )
    .bind(&[
        validated.name.into(),
        validated.url.into(),
        js_opt_string(validated.username),
        js_opt_string(password_enc),
        (if validated.enabled { 1 } else { 0 }).into(),
        validated.cadence.into(),
        validated.time.into(),
        js_i64(validated.weekday),
        js_i64(validated.utc_offset_minutes),
        js_i64(validated.keep_count),
        js_opt_i64(next),
        Utc::now().to_rfc3339().into(),
        id.clone().into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    let channel = load_channel(&db, &id).await?;
    Ok(Json(sanitized(&channel)))
}

#[worker::send]
pub async fn delete_channel(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    db.prepare("DELETE FROM backup_channels WHERE id = ?1")
        .bind(&[id.clone().into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    db.prepare("DELETE FROM backup_locks WHERE channel_id = ?1")
        .bind(&[id.clone().into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    db.prepare("DELETE FROM backup_runs WHERE channel_id = ?1")
        .bind(&[id.clone().into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    Ok(Json(json!({})))
}

#[worker::send]
pub async fn test_channel(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let channel = load_channel(&db, &id).await?;
    let key = backup_key(&env)?;
    let client = client_for(&channel, &key)?;
    client.test().await?;
    Ok(Json(json!({"ok": true})))
}

#[worker::send]
pub async fn run_now(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let channel = load_channel(&db, &id).await?;
    Ok(Json(run_channel(&env, &channel, "manual").await?))
}

#[worker::send]
pub async fn list_files(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let channel = load_channel(&db, &id).await?;
    let key = backup_key(&env)?;
    let client = client_for(&channel, &key)?;
    let files = client.list().await?;
    Ok(Json(json!({
        "data": files.into_iter().map(|name| json!({"name": name})).collect::<Vec<_>>()
    })))
}

#[worker::send]
pub async fn delete_file(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path((id, name)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    if !name.starts_with("warden-") || !name.ends_with(".enc") || name.contains('/') {
        return Err(AppError::BadRequest(
            "Invalid backup file name.".to_string(),
        ));
    }
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let channel = load_channel(&db, &id).await?;
    let key = backup_key(&env)?;
    let client = client_for(&channel, &key)?;
    client.delete(&name).await?;
    Ok(Json(json!({})))
}

#[derive(Deserialize)]
pub struct RestoreRequest {
    pub file: String,
    pub confirm: bool,
}

#[worker::send]
pub async fn restore(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Path(id): Path<String>,
    Json(input): Json<RestoreRequest>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    if !input.confirm {
        return Err(AppError::BadRequest(
            "Confirm that restoring replaces the current vault.".to_string(),
        ));
    }
    if !input.file.starts_with("warden-")
        || !input.file.ends_with(".enc")
        || input.file.contains('/')
    {
        return Err(AppError::BadRequest(
            "Invalid backup file name.".to_string(),
        ));
    }
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let channel = load_channel(&db, &id).await?;
    let key = backup_key(&env)?;
    let client = client_for(&channel, &key)?;

    let started = now_secs();
    let run_id = record_run_start(&db, &channel.id, "restore", started).await?;
    let outcome = async {
        // Keep a snapshot of the current state before replacing anything.
        create_snapshot(&env, &client, "warden-pre-restore").await?;
        let bytes = client.get(&input.file).await?;
        let envelope: Value = serde_json::from_slice(&bytes)
            .map_err(|_| AppError::BadRequest("This file is not a Warden backup.".to_string()))?;
        let payload = decrypt_backup(&key, &envelope)?;
        let tables = parse_payload(&payload)?;
        let counts = restore_payload(&db, &tables).await?;
        Ok::<Vec<(String, usize)>, AppError>(counts)
    }
    .await;

    match outcome {
        Ok(counts) => {
            if let Err(error) = record_run_finish(
                &db,
                &run_id,
                "success",
                Some(&input.file),
                None,
                None,
                now_secs(),
            )
            .await
            {
                log::warn!("Could not record restore result: {error}");
            }
            let restored: HashMap<String, usize> = counts.into_iter().collect();
            Ok(Json(json!({"restored": restored})))
        }
        Err(error) => {
            if let Err(record_error) = record_run_finish(
                &db,
                &run_id,
                "error",
                Some(&input.file),
                None,
                Some(&error.to_string()),
                now_secs(),
            )
            .await
            {
                log::warn!("Could not record restore result: {record_error}");
            }
            Err(error)
        }
    }
}

fn import_file_name(name: Option<&String>) -> String {
    let base = name
        .map(|value| value.rsplit(['/', '\\']).next().unwrap_or(value))
        .unwrap_or("offline-import.enc");
    let cleaned: String = base
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        "offline-import.enc".to_string()
    } else {
        cleaned.to_string()
    }
}

#[worker::send]
pub async fn import_backup(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    if body.is_empty() {
        return Err(AppError::BadRequest(
            "The uploaded backup file is empty.".to_string(),
        ));
    }
    if body.len() > MAX_IMPORT_BYTES {
        return Err(AppError::BadRequest(format!(
            "The uploaded backup is larger than {} MiB.",
            MAX_IMPORT_BYTES / (1024 * 1024)
        )));
    }
    let file_name = import_file_name(query.get("name"));
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let key = backup_key(&env)?;

    let started = now_secs();
    let run_id = record_run_start(&db, "", "import", started).await?;
    let outcome = async {
        let envelope: Value = serde_json::from_slice(&body)
            .map_err(|_| AppError::BadRequest("This file is not a Warden backup.".to_string()))?;
        let payload = decrypt_backup(&key, &envelope)?;
        let tables = parse_payload(&payload)?;
        restore_payload(&db, &tables).await
    }
    .await;

    match outcome {
        Ok(counts) => {
            if let Err(error) = record_run_finish(
                &db,
                &run_id,
                "success",
                Some(&file_name),
                Some(body.len() as i64),
                None,
                now_secs(),
            )
            .await
            {
                log::warn!("Could not record import result: {error}");
            }
            let restored: HashMap<String, usize> = counts.into_iter().collect();
            Ok(Json(json!({"restored": restored})))
        }
        Err(error) => {
            if let Err(record_error) = record_run_finish(
                &db,
                &run_id,
                "error",
                Some(&file_name),
                Some(body.len() as i64),
                Some(&error.to_string()),
                now_secs(),
            )
            .await
            {
                log::warn!("Could not record import result: {record_error}");
            }
            Err(error)
        }
    }
}

#[worker::send]
pub async fn runs(
    claims: Claims,
    headers: HeaderMap,
    State(env): State<Arc<Env>>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    require_admin(&claims, &headers, &env).await?;
    let db = db::get_db(&env)?;
    ensure_tables(&db).await?;
    let channel_id = query.get("channelId").cloned();
    let rows: Vec<Value> = match channel_id {
        Some(channel_id) => db
            .prepare(
                "SELECT * FROM backup_runs WHERE channel_id = ?1 ORDER BY started_at DESC LIMIT 50",
            )
            .bind(&[channel_id.into()])?
            .all()
            .await
            .map_err(|_| AppError::Database)?
            .results()
            .map_err(|_| AppError::Database)?,
        None => db
            .prepare("SELECT * FROM backup_runs ORDER BY started_at DESC LIMIT 50")
            .all()
            .await
            .map_err(|_| AppError::Database)?
            .results()
            .map_err(|_| AppError::Database)?,
    };
    Ok(Json(json!({"data": rows})))
}
