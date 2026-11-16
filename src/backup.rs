use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::{json, Value};
use wasm_bindgen::JsValue;
use worker::{D1Database, D1PreparedStatement, Env};

use crate::error::AppError;

pub const BACKUP_FORMAT: &str = "warden-backup";
pub const BACKUP_VERSION: u64 = 1;
const BACKUP_AAD: &[u8] = b"warden-backup:v1";
const MAX_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_ROWS_TOTAL: usize = 20_000;
const MAX_STATEMENTS: usize = 2_000;
const MAX_STATEMENT_BINDING_BYTES: usize = 2_000_000;
const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const MAX_BOUND_PARAMS: usize = 100;

pub struct TableSpec {
    pub name: &'static str,
    pub columns: &'static [&'static str],
}

/// Dependency order: parents first, so a restore can insert in this order and delete in reverse.
pub const TABLES: &[TableSpec] = &[
    TableSpec {
        name: "users",
        columns: &[
            "id",
            "name",
            "avatar_color",
            "email",
            "email_verified",
            "master_password_hash",
            "master_password_hint",
            "key",
            "private_key",
            "public_key",
            "kdf_type",
            "kdf_iterations",
            "totp_recover",
            "security_stamp",
            "created_at",
            "updated_at",
        ],
    },
    TableSpec {
        name: "folders",
        columns: &["id", "user_id", "name", "created_at", "updated_at"],
    },
    TableSpec {
        name: "ciphers",
        columns: &[
            "id",
            "user_id",
            "organization_id",
            "type",
            "data",
            "key",
            "favorite",
            "folder_id",
            "deleted_at",
            "archived_at",
            "created_at",
            "updated_at",
        ],
    },
    TableSpec {
        name: "sends",
        columns: &[
            "id",
            "user_id",
            "organization_id",
            "type",
            "name",
            "notes",
            "data",
            "key",
            "password_hash",
            "password_salt",
            "password_iter",
            "max_access_count",
            "access_count",
            "created_at",
            "updated_at",
            "expiration_date",
            "deletion_date",
            "disabled",
            "hide_email",
        ],
    },
    TableSpec {
        name: "send_files",
        columns: &[
            "id",
            "send_id",
            "user_id",
            "file_name",
            "size",
            "mime",
            "data_base64",
            "created_at",
            "updated_at",
        ],
    },
    TableSpec {
        name: "send_file_chunks",
        columns: &[
            "send_file_id",
            "chunk_index",
            "data_base64",
            "created_at",
            "updated_at",
        ],
    },
    TableSpec {
        name: "two_factor_authenticator",
        columns: &[
            "user_id",
            "enabled",
            "secret_enc",
            "created_at",
            "updated_at",
        ],
    },
    TableSpec {
        name: "devices",
        columns: &[
            "id",
            "user_id",
            "device_identifier",
            "device_name",
            "device_type",
            "remember_token_hash",
            "remember_expires_at",
            "created_at",
            "updated_at",
        ],
    },
    TableSpec {
        name: "registration_allowlist",
        columns: &["email", "enabled", "created_at", "updated_at"],
    },
];

pub fn now_secs() -> i64 {
    Utc::now().timestamp()
}

pub async fn ensure_tables(db: &D1Database) -> Result<(), AppError> {
    for sql in [
        "CREATE TABLE IF NOT EXISTS backup_channels (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            type TEXT NOT NULL DEFAULT 'webdav',
            url TEXT NOT NULL,
            username TEXT,
            password_enc TEXT,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            cadence TEXT NOT NULL DEFAULT 'daily',
            time TEXT NOT NULL DEFAULT '03:00',
            weekday INTEGER NOT NULL DEFAULT 1,
            utc_offset_minutes INTEGER NOT NULL DEFAULT 480,
            keep_count INTEGER NOT NULL DEFAULT 7,
            next_run_at INTEGER,
            last_run_at INTEGER,
            last_status TEXT,
            last_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        "CREATE TABLE IF NOT EXISTS backup_runs (
            id TEXT PRIMARY KEY NOT NULL,
            channel_id TEXT NOT NULL,
            trigger TEXT NOT NULL,
            status TEXT NOT NULL,
            file_name TEXT,
            size INTEGER,
            error TEXT,
            started_at INTEGER NOT NULL,
            finished_at INTEGER
        )",
        "CREATE INDEX IF NOT EXISTS idx_backup_runs_channel ON backup_runs(channel_id, started_at)",
        "CREATE TABLE IF NOT EXISTS backup_locks (
            channel_id TEXT PRIMARY KEY NOT NULL,
            locked_until INTEGER NOT NULL
        )",
    ] {
        db.prepare(sql)
            .run()
            .await
            .map_err(|_| AppError::Database)?;
    }
    Ok(())
}

fn decode_backup_key(secret: &str) -> Result<[u8; 32], AppError> {
    let bytes = general_purpose::STANDARD
        .decode(secret.trim())
        .map_err(|_| {
            AppError::BadRequest(
                "BACKUP_ENC_KEY must be a base64-encoded 32-byte key (openssl rand -base64 32)."
                    .to_string(),
            )
        })?;
    bytes.try_into().map_err(|_| {
        AppError::BadRequest(
            "BACKUP_ENC_KEY must be a base64-encoded 32-byte key (openssl rand -base64 32)."
                .to_string(),
        )
    })
}

pub fn backup_key(env: &Env) -> Result<[u8; 32], AppError> {
    let secret = env.secret("BACKUP_ENC_KEY").map_err(|_| {
        AppError::BadRequest(
            "BACKUP_ENC_KEY is not configured. Set the Worker secret before using backups."
                .to_string(),
        )
    })?;
    decode_backup_key(&secret.to_string())
}

pub fn encryption_ready(env: &Env) -> bool {
    backup_key(env).is_ok()
}

fn scalar_value(value: Value) -> Result<Value, AppError> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) | Value::Number(_) => Ok(value),
        _ => Err(AppError::BadRequest(
            "The backup contains a value that is not a scalar.".to_string(),
        )),
    }
}

fn value_to_binding(value: &Value) -> Result<JsValue, AppError> {
    match value {
        Value::Null => Ok(JsValue::NULL),
        Value::Bool(flag) => Ok(JsValue::from_bool(*flag)),
        Value::String(text) => Ok(JsValue::from_str(text)),
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                if integer.unsigned_abs() > 9_007_199_254_740_991 {
                    return Err(AppError::BadRequest(
                        "The backup contains an integer that is too large.".to_string(),
                    ));
                }
                Ok(JsValue::from_f64(integer as f64))
            } else if let Some(float) = number.as_f64() {
                if !float.is_finite() {
                    return Err(AppError::BadRequest(
                        "The backup contains a non-finite number.".to_string(),
                    ));
                }
                Ok(JsValue::from_f64(float))
            } else {
                Err(AppError::BadRequest(
                    "The backup contains an unsupported number.".to_string(),
                ))
            }
        }
        _ => Err(AppError::BadRequest(
            "The backup contains a value that is not a scalar.".to_string(),
        )),
    }
}

pub async fn export_payload(db: &D1Database) -> Result<Value, AppError> {
    let mut selects = Vec::with_capacity(TABLES.len());
    for table in TABLES {
        selects.push(db.prepare(format!(
            "SELECT {} FROM {}",
            table.columns.join(", "),
            table.name
        )));
    }
    // One batch runs as a single D1 transaction, so all tables come from the same snapshot.
    let results = db.batch(selects).await.map_err(|error| {
        log::error!("Backup select batch failed: {error}");
        AppError::Database
    })?;
    if results.len() != TABLES.len() {
        return Err(AppError::Internal);
    }

    let mut tables = Vec::with_capacity(TABLES.len());
    let mut total_rows = 0usize;
    for (table, result) in TABLES.iter().zip(results.into_iter()) {
        let rows: Vec<Value> = result.results::<Value>().map_err(|_| AppError::Database)?;
        total_rows += rows.len();
        if total_rows > MAX_ROWS_TOTAL {
            return Err(AppError::BadRequest(format!(
                "The vault has more than {MAX_ROWS_TOTAL} rows; use `wrangler d1 export` for a full offline copy."
            )));
        }
        let mut values = Vec::with_capacity(rows.len());
        for row in rows {
            let object = row.as_object().ok_or(AppError::Internal)?;
            let mut columns = Vec::with_capacity(table.columns.len());
            for column in table.columns {
                columns.push(scalar_value(
                    object.get(*column).cloned().unwrap_or(Value::Null),
                )?);
            }
            values.push(Value::Array(columns));
        }
        tables.push(json!({
            "name": table.name,
            "columns": table.columns,
            "rows": values,
        }));
    }

    Ok(json!({
        "format": BACKUP_FORMAT,
        "version": BACKUP_VERSION,
        "createdAt": now_secs(),
        "tables": tables,
    }))
}

pub fn serialize_payload(payload: &Value) -> Result<Vec<u8>, AppError> {
    let bytes = serde_json::to_vec(payload).map_err(|_| AppError::Internal)?;
    if bytes.len() > MAX_PLAINTEXT_BYTES {
        return Err(AppError::BadRequest(format!(
            "The vault is larger than {} MiB; use `wrangler d1 export` for a full offline copy.",
            MAX_PLAINTEXT_BYTES / (1024 * 1024)
        )));
    }
    Ok(bytes)
}

pub fn encrypt_backup(key: &[u8; 32], plaintext: &[u8]) -> Result<Value, AppError> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut iv = [0u8; 12];
    OsRng.fill_bytes(&mut iv);
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: plaintext,
                aad: BACKUP_AAD,
            },
        )
        .map_err(|_| AppError::Crypto("Could not encrypt the backup".to_string()))?;
    Ok(json!({
        "format": BACKUP_FORMAT,
        "version": BACKUP_VERSION,
        "alg": "AES-256-GCM",
        "createdAt": now_secs(),
        "iv": general_purpose::STANDARD.encode(iv),
        "data": general_purpose::STANDARD.encode(encrypted),
    }))
}

pub fn decrypt_backup(key: &[u8; 32], envelope: &Value) -> Result<Value, AppError> {
    let invalid = || AppError::BadRequest("This file is not a Warden backup.".to_string());
    let object = envelope.as_object().ok_or_else(invalid)?;
    if object.get("format").and_then(Value::as_str) != Some(BACKUP_FORMAT)
        || object.get("version").and_then(Value::as_u64) != Some(BACKUP_VERSION)
        || object.get("alg").and_then(Value::as_str) != Some("AES-256-GCM")
    {
        return Err(invalid());
    }
    let iv = object
        .get("iv")
        .and_then(Value::as_str)
        .and_then(|text| general_purpose::STANDARD.decode(text).ok())
        .filter(|bytes| bytes.len() == 12)
        .ok_or_else(invalid)?;
    let data = object
        .get("data")
        .and_then(Value::as_str)
        .and_then(|text| general_purpose::STANDARD.decode(text).ok())
        .filter(|bytes| !bytes.is_empty() && bytes.len() <= MAX_FILE_BYTES)
        .ok_or_else(invalid)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &data,
                aad: BACKUP_AAD,
            },
        )
        .map_err(|_| {
            AppError::BadRequest(
                "The backup cannot be decrypted. Check BACKUP_ENC_KEY.".to_string(),
            )
        })?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(invalid());
    }
    let payload: Value = serde_json::from_slice(&plaintext)
        .map_err(|_| AppError::BadRequest("The backup contents are damaged.".to_string()))?;
    Ok(payload)
}

pub struct ParsedTable {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

pub fn parse_payload(payload: &Value) -> Result<Vec<ParsedTable>, AppError> {
    let invalid = |message: &str| AppError::BadRequest(message.to_string());
    let object = payload
        .as_object()
        .ok_or_else(|| invalid("The backup contents are damaged."))?;
    if object.get("format").and_then(Value::as_str) != Some(BACKUP_FORMAT)
        || object.get("version").and_then(Value::as_u64) != Some(BACKUP_VERSION)
    {
        return Err(invalid(
            "The backup format is not supported by this deployment.",
        ));
    }
    let raw_tables = object
        .get("tables")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("The backup has no tables."))?;

    let mut found: Vec<Option<ParsedTable>> = (0..TABLES.len()).map(|_| None).collect();
    let mut total_rows = 0usize;
    for raw in raw_tables {
        let table = raw
            .as_object()
            .ok_or_else(|| invalid("The backup has an invalid table."))?;
        let name = table
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("The backup has an invalid table name."))?;
        let index = TABLES
            .iter()
            .position(|spec| spec.name == name)
            .ok_or_else(|| invalid(&format!("The backup contains an unknown table: {name}.")))?;
        if found[index].is_some() {
            return Err(invalid("The backup contains a duplicate table."));
        }
        let columns: Vec<String> = table
            .get("columns")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("The backup has invalid columns."))?
            .iter()
            .map(|column| {
                column
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| invalid("The backup has an invalid column name."))
            })
            .collect::<Result<_, _>>()?;
        let expected = TABLES[index].columns;
        if columns.len() != expected.len()
            || !expected
                .iter()
                .all(|column| columns.iter().any(|item| item == column))
        {
            return Err(invalid(
                "The backup schema does not match this deployment; restore it with `wrangler d1 execute` instead.",
            ));
        }
        let raw_rows = table
            .get("rows")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("The backup has invalid rows."))?;
        total_rows += raw_rows.len();
        if total_rows > MAX_ROWS_TOTAL {
            return Err(invalid("The backup contains too many rows."));
        }
        let mut rows = Vec::with_capacity(raw_rows.len());
        for row in raw_rows {
            let values = row
                .as_array()
                .ok_or_else(|| invalid("The backup has an invalid row."))?;
            if values.len() != expected.len() {
                return Err(invalid(
                    "The backup has a row with the wrong number of columns.",
                ));
            }
            rows.push(
                values
                    .iter()
                    .cloned()
                    .map(scalar_value)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        found[index] = Some(ParsedTable {
            name: name.to_string(),
            columns,
            rows,
        });
    }

    let mut tables = Vec::with_capacity(TABLES.len());
    for (index, table) in found.into_iter().enumerate() {
        tables.push(
            table.ok_or_else(|| {
                invalid(&format!("The backup is missing {}.", TABLES[index].name))
            })?,
        );
    }
    Ok(tables)
}

pub async fn restore_payload(
    db: &D1Database,
    tables: &[ParsedTable],
) -> Result<Vec<(String, usize)>, AppError> {
    let mut statements: Vec<D1PreparedStatement> = Vec::new();
    for table in TABLES.iter().rev() {
        statements.push(db.prepare(format!("DELETE FROM {}", table.name)));
    }

    let mut counts = Vec::with_capacity(tables.len());
    for (index, parsed) in tables.iter().enumerate() {
        let spec = &TABLES[index];
        if parsed.name != spec.name {
            return Err(AppError::Internal);
        }
        // Reorder the file's values into this deployment's column order.
        let order: Vec<usize> = spec
            .columns
            .iter()
            .map(|column| {
                parsed
                    .columns
                    .iter()
                    .position(|item| item == column)
                    .ok_or(AppError::Internal)
            })
            .collect::<Result<_, _>>()?;
        let per_statement = (MAX_BOUND_PARAMS / spec.columns.len()).max(1);
        let mut start = 0usize;
        while start < parsed.rows.len() {
            let mut end = start;
            let mut bytes = 0usize;
            while end < parsed.rows.len() && end - start < per_statement {
                let row_bytes = parsed.rows[end]
                    .iter()
                    .map(|value| match value {
                        Value::String(text) => text.len() + 1,
                        Value::Null => 1,
                        _ => 16,
                    })
                    .sum::<usize>();
                if end > start && bytes + row_bytes > MAX_STATEMENT_BINDING_BYTES {
                    break;
                }
                bytes += row_bytes;
                end += 1;
            }
            let chunk = &parsed.rows[start..end];
            let placeholders = chunk
                .iter()
                .map(|_| format!("({})", vec!["?"; spec.columns.len()].join(", ")))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "INSERT INTO {} ({}) VALUES {}",
                spec.name,
                spec.columns.join(", "),
                placeholders
            );
            let mut bindings = Vec::with_capacity(chunk.len() * spec.columns.len());
            for row in chunk {
                for &position in &order {
                    bindings.push(value_to_binding(&row[position])?);
                }
            }
            statements.push(db.prepare(sql).bind(&bindings).map_err(AppError::Worker)?);
            start = end;
            if statements.len() > MAX_STATEMENTS {
                return Err(AppError::BadRequest(
                    "The backup is too large to restore in one transaction; restore it with `wrangler d1 execute` instead."
                        .to_string(),
                ));
            }
        }
        counts.push((spec.name.to_string(), parsed.rows.len()));
        if statements.len() > MAX_STATEMENTS {
            return Err(AppError::BadRequest(
                "The backup is too large to restore in one transaction; restore it with `wrangler d1 execute` instead."
                    .to_string(),
            ));
        }
    }

    // One batch is one D1 transaction: either the whole vault is replaced or nothing changes.
    db.batch(statements).await.map_err(|error| {
        log::error!("Backup restore batch failed: {error}");
        AppError::BadRequest(
            "The restore failed and was rolled back. Check the Worker logs for the first failing table."
                .to_string(),
        )
    })?;
    Ok(counts)
}

pub fn parse_time(value: &str) -> Result<(i64, i64), AppError> {
    let (hour, minute) = value
        .split_once(':')
        .ok_or_else(|| AppError::BadRequest("Use HH:MM for the backup time.".to_string()))?;
    let hour: i64 = hour
        .parse()
        .map_err(|_| AppError::BadRequest("Use HH:MM for the backup time.".to_string()))?;
    let minute: i64 = minute
        .parse()
        .map_err(|_| AppError::BadRequest("Use HH:MM for the backup time.".to_string()))?;
    if !(0..24).contains(&hour) || !(0..60).contains(&minute) {
        return Err(AppError::BadRequest(
            "Use a time between 00:00 and 23:59.".to_string(),
        ));
    }
    Ok((hour, minute))
}

/// Next scheduled run strictly after `now`. The offset is fixed, so DST is not handled.
pub fn next_run_after(
    now: i64,
    cadence: &str,
    time: &str,
    weekday: i64,
    utc_offset_minutes: i64,
) -> Result<i64, AppError> {
    if !matches!(cadence, "daily" | "weekly") {
        return Err(AppError::BadRequest(
            "Choose a daily or weekly schedule.".to_string(),
        ));
    }
    if !(0..=6).contains(&weekday) {
        return Err(AppError::BadRequest(
            "Choose a weekday from Sunday to Saturday.".to_string(),
        ));
    }
    if !(-720..=840).contains(&utc_offset_minutes) {
        return Err(AppError::BadRequest(
            "The UTC offset must be between -720 and 840 minutes.".to_string(),
        ));
    }
    let (hour, minute) = parse_time(time)?;
    let target = hour * 3600 + minute * 60;
    let offset = utc_offset_minutes * 60;
    let local_now = now + offset;
    let local_day = local_now.div_euclid(86_400);
    let local_second = local_now.rem_euclid(86_400);
    let mut day = if target > local_second {
        local_day
    } else {
        local_day + 1
    };
    if cadence == "weekly" {
        for _ in 0..7 {
            // 1970-01-01 was a Thursday (4 in a Sunday-first week).
            if (day + 4).rem_euclid(7) == weekday {
                break;
            }
            day += 1;
        }
    }
    Ok(day * 86_400 + target - offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty_payload() -> Value {
        json!({
            "format": BACKUP_FORMAT,
            "version": BACKUP_VERSION,
            "createdAt": 0,
            "tables": TABLES
                .iter()
                .map(|table| json!({
                    "name": table.name,
                    "columns": table.columns,
                    "rows": [],
                }))
                .collect::<Vec<_>>(),
        })
    }

    #[test]
    fn daily_schedule_respects_utc_offset() {
        // 2026-01-01T00:00:00Z is 08:00 at UTC+08:00, so 03:00 has already passed today.
        let next = next_run_after(1_767_225_600, "daily", "03:00", 1, 480).expect("schedule");
        assert_eq!(next, 1_767_294_000);
    }

    #[test]
    fn weekly_schedule_finds_next_weekday() {
        // 2026-01-01T00:00:00Z is a Thursday; the next Monday 03:00 UTC is 2026-01-05.
        let next = next_run_after(1_767_225_600, "weekly", "03:00", 1, 0).expect("schedule");
        assert_eq!(next, 1_767_582_000);
    }

    #[test]
    fn rejects_invalid_time_and_weekday() {
        assert!(parse_time("24:00").is_err());
        assert!(next_run_after(1_767_225_600, "weekly", "03:00", 7, 0).is_err());
        assert!(next_run_after(1_767_225_600, "hourly", "03:00", 1, 0).is_err());
    }

    #[test]
    fn encryption_round_trip_and_authentication_detection() {
        let key = [7u8; 32];
        let envelope = encrypt_backup(&key, br#"{"ok":true}"#).expect("encrypt");
        let plain = decrypt_backup(&key, &envelope).expect("decrypt");
        assert_eq!(plain, json!({"ok": true}));

        let mut tampered = envelope.clone();
        tampered["data"] = json!("AAAA");
        assert!(decrypt_backup(&key, &tampered).is_err());
        assert!(decrypt_backup(&[8u8; 32], &envelope).is_err());
    }

    #[test]
    fn parses_every_expected_table_and_rejects_unknown_ones() {
        let parsed = parse_payload(&empty_payload()).expect("parse");
        assert_eq!(parsed.len(), TABLES.len());

        let mut payload = empty_payload();
        payload["tables"].as_array_mut().unwrap().push(json!({
            "name": "secrets",
            "columns": ["id"],
            "rows": [],
        }));
        assert!(parse_payload(&payload).is_err());
    }
}
