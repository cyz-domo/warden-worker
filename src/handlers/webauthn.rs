use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use worker::Env;

use crate::auth::Claims;
use crate::db;
use crate::error::AppError;
use crate::handlers::two_factor::PasswordOrOtpData;
use crate::models::user::User;
use crate::webauthn;

const MAX_PASSKEYS: i64 = 5;
const MAX_ENCRYPTED_KEY_LEN: usize = 2000;
const UPDATE_KEYSET_CHALLENGE_KIND: &str = "update_keyset";

pub const PRF_STATUS_ENABLED: i64 = 0;
pub const PRF_STATUS_SUPPORTED: i64 = 1;
pub const PRF_STATUS_UNSUPPORTED: i64 = 2;

#[derive(Debug, Clone)]
pub struct PrfOption {
    encrypted_private_key: String,
    encrypted_user_key: String,
    credential_id: String,
}

impl PrfOption {
    fn from_columns(
        credential_id: impl Into<String>,
        encrypted_private_key: Option<&str>,
        encrypted_user_key: Option<&str>,
    ) -> Option<Self> {
        let credential_id = credential_id.into();
        let encrypted_private_key = encrypted_private_key.filter(|value| !value.is_empty())?;
        let encrypted_user_key = encrypted_user_key.filter(|value| !value.is_empty())?;
        if credential_id.is_empty() {
            return None;
        }
        Some(Self {
            encrypted_private_key: encrypted_private_key.to_string(),
            encrypted_user_key: encrypted_user_key.to_string(),
            credential_id,
        })
    }

    pub fn from_row(row: &Value) -> Option<Self> {
        Self::from_columns(
            row.get("credential_id")?.as_str()?,
            row.get("encrypted_private_key")?.as_str(),
            row.get("encrypted_user_key")?.as_str(),
        )
    }

    pub fn identity_json(&self) -> Value {
        json!({
            "EncryptedPrivateKey": self.encrypted_private_key,
            "EncryptedUserKey": self.encrypted_user_key,
            "CredentialId": self.credential_id,
            "Transports": []
        })
    }

    pub fn sync_json(&self) -> Value {
        json!({
            "encryptedPrivateKey": self.encrypted_private_key,
            "encryptedUserKey": self.encrypted_user_key,
            "credentialId": self.credential_id,
            "transports": []
        })
    }
}

pub struct PasskeyLoginResult {
    pub user: User,
    pub prf_option: Option<PrfOption>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssertionEnvelope {
    id: Option<String>,
    #[serde(default)]
    response: AssertionResponse,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssertionResponse {
    #[serde(default)]
    authenticator_data: String,
    #[serde(default)]
    signature: String,
    #[serde(default, alias = "clientDataJSON", alias = "clientDataJson")]
    client_data_json: String,
    #[serde(default)]
    user_handle: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttestationEnvelope {
    id: Option<String>,
    #[serde(default)]
    response: AttestationResponse,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttestationResponse {
    #[serde(default, alias = "AttestationObject")]
    attestation_object: String,
    #[serde(default, alias = "clientDataJson", alias = "clientDataJSON")]
    client_data_json: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveCredentialRequest {
    token: String,
    device_response: AttestationEnvelope,
    name: Option<String>,
    #[serde(default)]
    supports_prf: Option<bool>,
    #[serde(default)]
    encrypted_user_key: Option<String>,
    #[serde(default)]
    encrypted_public_key: Option<String>,
    #[serde(default)]
    encrypted_private_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCredentialRequest {
    token: String,
    device_response: AssertionEnvelope,
    encrypted_user_key: String,
    encrypted_public_key: String,
    encrypted_private_key: String,
}

fn prepare_prf_keys(
    supports_prf: bool,
    encrypted_user_key: Option<&str>,
    encrypted_public_key: Option<&str>,
    encrypted_private_key: Option<&str>,
) -> Result<(i64, [Option<String>; 3]), AppError> {
    let raw = [
        encrypted_user_key,
        encrypted_public_key,
        encrypted_private_key,
    ];
    if raw
        .iter()
        .flatten()
        .any(|value| value.len() > MAX_ENCRYPTED_KEY_LEN)
    {
        return Err(AppError::BadRequest(
            "Encrypted passkey key is too long".to_string(),
        ));
    }

    let keys = raw.map(|value| {
        value
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
    });
    let present = keys.iter().filter(|value| value.is_some()).count();
    if present != 0 && present != 3 {
        return Err(AppError::BadRequest(
            "PRF key set must include all keys".to_string(),
        ));
    }

    let status = if !supports_prf {
        PRF_STATUS_UNSUPPORTED
    } else if present == 3 {
        PRF_STATUS_ENABLED
    } else {
        PRF_STATUS_SUPPORTED
    };
    Ok((status, keys))
}

#[worker::send]
pub async fn login_assertion_options(
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let client = headers
        .get("cf-connecting-ip")
        .and_then(|value| value.to_str().ok());
    if !crate::ratelimit::prelogin_allowed(client) {
        return Err(AppError::TooManyRequests);
    }
    let db = db::get_db(&env)?;
    webauthn::ensure_tables(&db).await?;
    let rp = webauthn::rp_config(&env, &headers)?;
    let _ = webauthn::cleanup_expired_challenges(&db).await;

    let challenge = webauthn::random_challenge();
    let token = webauthn::random_token();
    webauthn::store_challenge(
        &db,
        None,
        "login",
        &challenge,
        Some(&webauthn::sha256_hex(&token)),
        webauthn::CHALLENGE_TTL_LOGIN_SECS,
    )
    .await?;

    Ok(Json(json!({
        "options": {
            "challenge": challenge,
            "rpId": rp.rp_id,
            "timeout": 60000,
            "allowCredentials": [],
            "userVerification": "required"
        },
        "token": token
    })))
}

#[worker::send]
pub async fn attestation_options(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Json(payload): Json<PasswordOrOtpData>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    payload.validate(&db, &claims.sub, &env).await?;
    webauthn::ensure_tables(&db).await?;
    let rp = webauthn::rp_config(&env, &headers)?;
    let _ = webauthn::cleanup_expired_challenges(&db).await;

    let count: Option<i64> = db
        .prepare("SELECT COUNT(1) AS count FROM webauthn_credentials WHERE user_id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(Some("count"))
        .await
        .map_err(|_| AppError::Database)?;
    if count.unwrap_or(0) >= MAX_PASSKEYS {
        return Err(AppError::BadRequest(
            "Maximum number of passkeys reached".to_string(),
        ));
    }

    let user: Option<Value> = db
        .prepare("SELECT email, name FROM users WHERE id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?;
    let user = user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let email = user
        .get("email")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let display_name = user
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .unwrap_or(&email)
        .to_string();

    let mut exclude = Vec::new();
    let existing = db
        .prepare("SELECT credential_id FROM webauthn_credentials WHERE user_id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results::<Value>()
        .map_err(|_| AppError::Database)?;
    for row in existing {
        if let Some(id) = row.get("credential_id").and_then(Value::as_str) {
            exclude.push(json!({ "type": "public-key", "id": id }));
        }
    }

    let challenge = webauthn::random_challenge();
    let token = webauthn::random_token();
    webauthn::store_challenge(
        &db,
        Some(&claims.sub),
        "register",
        &challenge,
        Some(&webauthn::sha256_hex(&token)),
        webauthn::CHALLENGE_TTL_REGISTER_SECS,
    )
    .await?;

    Ok(Json(json!({
        "options": {
            "rp": { "id": rp.rp_id, "name": rp.rp_name },
            "user": {
                "id": webauthn::b64url_encode(claims.sub.as_bytes()),
                "name": email,
                "displayName": display_name
            },
            "challenge": challenge,
            "pubKeyCredParams": [{ "type": "public-key", "alg": -7 }],
            "timeout": 60000,
            "excludeCredentials": exclude,
            "authenticatorSelection": {
                "residentKey": "required",
                "requireResidentKey": true,
                "userVerification": "required"
            },
            "attestation": "none",
            "extensions": { "prf": {} }
        },
        "token": token
    })))
}

#[worker::send]
pub async fn save_credential(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Json(payload): Json<SaveCredentialRequest>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    webauthn::ensure_tables(&db).await?;
    let rp = webauthn::rp_config(&env, &headers)?;

    let challenge_row =
        webauthn::load_challenge_by_token(&db, &webauthn::sha256_hex(&payload.token), "register")
            .await?
            .ok_or_else(|| {
                AppError::BadRequest("Invalid or expired registration token".to_string())
            })?;
    if challenge_row.user_id.as_deref() != Some(claims.sub.as_str()) {
        return Err(AppError::Unauthorized(
            "Registration token mismatch".to_string(),
        ));
    }

    let client_data_json = webauthn::b64_decode(&payload.device_response.response.client_data_json)
        .map_err(|_| AppError::BadRequest("Invalid client data".to_string()))?;
    let client_data = webauthn::parse_client_data(&client_data_json)?;
    webauthn::validate_client_data(
        &client_data,
        "webauthn.create",
        &challenge_row.challenge,
        &rp.origin,
    )?;

    let attestation_bytes =
        webauthn::b64_decode(&payload.device_response.response.attestation_object)
            .map_err(|_| AppError::BadRequest("Invalid attestation object".to_string()))?;
    let attestation = webauthn::parse_attestation_object(&attestation_bytes)?;
    let auth_data = webauthn::parse_auth_data(&attestation.auth_data, true)?;
    webauthn::validate_rp_id_hash(&auth_data, &rp.rp_id)?;
    webauthn::validate_user_presence(&auth_data, true)?;
    let credential = auth_data
        .credential
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Missing credential".to_string()))?;

    let client_data_hash = {
        use sha2::{Digest, Sha256};
        Sha256::digest(&client_data_json).to_vec()
    };
    webauthn::verify_attestation(&attestation, &client_data_hash, credential)?;

    let credential_id = webauthn::b64url_encode(&credential.credential_id);
    if let Some(device_id) = payload.device_response.id.as_deref() {
        if webauthn::normalize_credential_id(device_id)? != credential_id {
            return Err(AppError::BadRequest(
                "Credential id does not match attestation".to_string(),
            ));
        }
    }

    let duplicate: Option<i64> = db
        .prepare("SELECT 1 AS ok FROM webauthn_credentials WHERE credential_id = ?1")
        .bind(&[credential_id.clone().into()])?
        .first(Some("ok"))
        .await
        .map_err(|_| AppError::Database)?;
    if duplicate.is_some() {
        return Err(AppError::BadRequest(
            "This passkey is already registered".to_string(),
        ));
    }

    let count: Option<i64> = db
        .prepare("SELECT COUNT(1) AS count FROM webauthn_credentials WHERE user_id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(Some("count"))
        .await
        .map_err(|_| AppError::Database)?;
    if count.unwrap_or(0) >= MAX_PASSKEYS {
        return Err(AppError::BadRequest(
            "Maximum number of passkeys reached".to_string(),
        ));
    }

    let (prf_status, prf_keys) = prepare_prf_keys(
        payload.supports_prf.unwrap_or(false),
        payload.encrypted_user_key.as_deref(),
        payload.encrypted_public_key.as_deref(),
        payload.encrypted_private_key.as_deref(),
    )?;
    let [encrypted_user_key, encrypted_public_key, encrypted_private_key] = prf_keys;

    let now = Utc::now().to_rfc3339();
    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("Passkey");
    db.prepare(
        "INSERT INTO webauthn_credentials
            (id, user_id, name, credential_id, public_key, sign_count, transports, prf_status,
             encrypted_user_key, encrypted_public_key, encrypted_private_key, created_at, updated_at, last_used_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10, ?11, ?11, NULL)",
    )
    .bind(&[
        Uuid::new_v4().to_string().into(),
        claims.sub.clone().into(),
        name.into(),
        credential_id.into(),
        webauthn::b64_std_encode(&credential.public_key).into(),
        (auth_data.sign_count as f64).into(),
        (prf_status as f64).into(),
        encrypted_user_key
            .map(|v| v.into())
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
        encrypted_public_key
            .map(|v| v.into())
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
        encrypted_private_key
            .map(|v| v.into())
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
        now.into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    webauthn::consume_challenge(&db, &challenge_row.id).await?;

    Ok(Json(json!({})))
}

#[worker::send]
pub async fn list_credentials(
    claims: Claims,
    State(env): State<Arc<Env>>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    webauthn::ensure_tables(&db).await?;
    let rows = db
        .prepare(
            "SELECT id, name, prf_status, encrypted_public_key, encrypted_user_key
             FROM webauthn_credentials WHERE user_id = ?1 ORDER BY created_at",
        )
        .bind(&[claims.sub.clone().into()])?
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results::<Value>()
        .map_err(|_| AppError::Database)?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            json!({
                "Id": row.get("id").cloned().unwrap_or(Value::Null),
                "Name": row.get("name").cloned().unwrap_or(Value::Null),
                "PrfStatus": row.get("prf_status").cloned().unwrap_or(json!(2)),
                "EncryptedPublicKey": row.get("encrypted_public_key").cloned().unwrap_or(Value::Null),
                "EncryptedUserKey": row.get("encrypted_user_key").cloned().unwrap_or(Value::Null)
            })
        })
        .collect();

    Ok(Json(json!({
        "object": "list",
        "data": data,
        "continuationToken": null
    })))
}

#[worker::send]
pub async fn delete_credential(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Path(id): Path<String>,
    Json(payload): Json<PasswordOrOtpData>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    payload.validate(&db, &claims.sub, &env).await?;
    webauthn::ensure_tables(&db).await?;
    let result = db
        .prepare("DELETE FROM webauthn_credentials WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.into(), claims.sub.clone().into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    let changes = result
        .meta()
        .map_err(|_| AppError::Database)?
        .and_then(|meta| meta.changes)
        .unwrap_or(0);
    if changes == 0 {
        return Err(AppError::NotFound("Passkey not found".to_string()));
    }
    Ok(Json(json!({})))
}

#[worker::send]
pub async fn update_credential(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Json(payload): Json<UpdateCredentialRequest>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    webauthn::ensure_tables(&db).await?;
    let rp = webauthn::rp_config(&env, &headers)?;
    let challenge_row = webauthn::load_challenge_by_token(
        &db,
        &webauthn::sha256_hex(&payload.token),
        UPDATE_KEYSET_CHALLENGE_KIND,
    )
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid or expired passkey token".to_string()))?;
    if challenge_row.user_id.as_deref() != Some(claims.sub.as_str()) {
        return Err(AppError::Unauthorized("Passkey token mismatch".to_string()));
    }

    let verified = verify_assertion(
        &db,
        &rp,
        &challenge_row,
        &payload.device_response,
        Some(claims.sub.as_str()),
    )
    .await?;
    if verified.prf_status == PRF_STATUS_UNSUPPORTED {
        return Err(AppError::BadRequest(
            "This passkey does not support PRF".to_string(),
        ));
    }

    let (status, prf_keys) = prepare_prf_keys(
        true,
        Some(&payload.encrypted_user_key),
        Some(&payload.encrypted_public_key),
        Some(&payload.encrypted_private_key),
    )?;
    if status != PRF_STATUS_ENABLED {
        return Err(AppError::BadRequest(
            "PRF key set must include all keys".to_string(),
        ));
    }
    let [encrypted_user_key, encrypted_public_key, encrypted_private_key] = prf_keys;
    let now = Utc::now().to_rfc3339();

    db.prepare(
        "UPDATE webauthn_credentials
         SET encrypted_user_key = ?1, encrypted_public_key = ?2, encrypted_private_key = ?3,
             prf_status = ?4, sign_count = ?5, last_used_at = ?6, updated_at = ?6
         WHERE user_id = ?7 AND credential_id = ?8",
    )
    .bind(&[
        encrypted_user_key
            .map(|v| v.into())
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
        encrypted_public_key
            .map(|v| v.into())
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
        encrypted_private_key
            .map(|v| v.into())
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
        (PRF_STATUS_ENABLED as f64).into(),
        (verified.sign_count as f64).into(),
        now.into(),
        verified.user_id.into(),
        verified.credential_id.into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    webauthn::consume_challenge(&db, &challenge_row.id).await?;

    Ok(Json(json!({})))
}

#[worker::send]
pub async fn assertion_options(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Json(payload): Json<PasswordOrOtpData>,
) -> Result<Json<Value>, AppError> {
    let db = db::get_db(&env)?;
    payload.validate(&db, &claims.sub, &env).await?;
    webauthn::ensure_tables(&db).await?;
    let rp = webauthn::rp_config(&env, &headers)?;
    let _ = webauthn::cleanup_expired_challenges(&db).await;

    let challenge = webauthn::random_challenge();
    let token = webauthn::random_token();
    webauthn::store_challenge(
        &db,
        Some(&claims.sub),
        UPDATE_KEYSET_CHALLENGE_KIND,
        &challenge,
        Some(&webauthn::sha256_hex(&token)),
        webauthn::CHALLENGE_TTL_LOGIN_SECS,
    )
    .await?;

    Ok(Json(json!({
        "options": {
            "challenge": challenge,
            "rpId": rp.rp_id,
            "timeout": 60000,
            "allowCredentials": [],
            "userVerification": "required"
        },
        "token": token
    })))
}

struct VerifiedAssertion {
    user_id: String,
    credential_id: String,
    sign_count: i64,
    prf_status: i64,
    encrypted_user_key: Option<String>,
    encrypted_private_key: Option<String>,
}

async fn verify_assertion(
    db: &worker::D1Database,
    rp: &webauthn::RpConfig,
    challenge_row: &webauthn::ChallengeRow,
    assertion: &AssertionEnvelope,
    expected_user_id: Option<&str>,
) -> Result<VerifiedAssertion, AppError> {
    let client_data_json = webauthn::b64_decode(&assertion.response.client_data_json)
        .map_err(|_| AppError::BadRequest("Invalid client data".to_string()))?;
    let client_data = webauthn::parse_client_data(&client_data_json)?;
    webauthn::validate_client_data(
        &client_data,
        "webauthn.get",
        &challenge_row.challenge,
        &rp.origin,
    )?;

    let auth_data_bytes = webauthn::b64_decode(&assertion.response.authenticator_data)
        .map_err(|_| AppError::BadRequest("Invalid authenticator data".to_string()))?;
    let auth_data = webauthn::parse_auth_data(&auth_data_bytes, false)?;
    webauthn::validate_rp_id_hash(&auth_data, &rp.rp_id)?;
    webauthn::validate_user_presence(&auth_data, true)?;

    let user_handle = assertion
        .response
        .user_handle
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Unauthorized(
                "This passkey cannot identify an account, use another sign-in method".to_string(),
            )
        })?;
    let user_id = String::from_utf8(webauthn::b64_decode(user_handle)?)
        .map_err(|_| AppError::Unauthorized("Invalid passkey user handle".to_string()))?;
    if let Some(expected_user_id) = expected_user_id {
        if expected_user_id != user_id.as_str() {
            return Err(AppError::Unauthorized("Passkey token mismatch".to_string()));
        }
    }

    let credential_id = assertion
        .id
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("Missing credential id".to_string()))
        .and_then(webauthn::normalize_credential_id)?;
    let row: Option<Value> = db
        .prepare(
            "SELECT public_key, sign_count, prf_status, encrypted_user_key, encrypted_private_key
             FROM webauthn_credentials WHERE user_id = ?1 AND credential_id = ?2",
        )
        .bind(&[user_id.clone().into(), credential_id.clone().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?;
    let row = row.ok_or_else(|| AppError::Unauthorized("Passkey is not registered".to_string()))?;
    let public_key = row
        .get("public_key")
        .and_then(Value::as_str)
        .ok_or(AppError::Internal)?;
    let stored_count = row.get("sign_count").and_then(Value::as_i64).unwrap_or(0);
    let signature = webauthn::b64_decode(&assertion.response.signature)?;
    webauthn::verify_assertion_signature(
        &webauthn::b64_decode(public_key)?,
        &auth_data_bytes,
        &client_data_json,
        &signature,
    )?;
    let sign_count = check_sign_count(stored_count, auth_data.sign_count)?;

    Ok(VerifiedAssertion {
        user_id,
        credential_id,
        sign_count,
        prf_status: row
            .get("prf_status")
            .and_then(Value::as_i64)
            .unwrap_or(PRF_STATUS_UNSUPPORTED),
        encrypted_user_key: row
            .get("encrypted_user_key")
            .and_then(Value::as_str)
            .map(str::to_string),
        encrypted_private_key: row
            .get("encrypted_private_key")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn check_sign_count(stored: i64, received: u32) -> Result<i64, AppError> {
    let received = received as i64;
    if received != 0 && stored != 0 && received <= stored {
        return Err(AppError::Unauthorized(
            "WebAuthn signature counter did not advance".to_string(),
        ));
    }
    Ok(received.max(stored))
}

pub async fn verify_passkey_login(
    env: &Env,
    db: &worker::D1Database,
    headers: &HeaderMap,
    token: &str,
    device_response: &str,
) -> Result<PasskeyLoginResult, AppError> {
    webauthn::ensure_tables(db).await?;
    let rp = webauthn::rp_config(env, headers)?;
    let challenge_row =
        webauthn::load_challenge_by_token(db, &webauthn::sha256_hex(token), "login")
            .await?
            .ok_or_else(|| {
                AppError::Unauthorized("Invalid or expired passkey token".to_string())
            })?;

    let assertion: AssertionEnvelope = serde_json::from_str(device_response)
        .map_err(|_| AppError::BadRequest("Invalid passkey assertion".to_string()))?;
    let verified = verify_assertion(db, &rp, &challenge_row, &assertion, None).await?;

    let now = Utc::now().to_rfc3339();
    db.prepare(
        "UPDATE webauthn_credentials SET sign_count = ?1, last_used_at = ?2, updated_at = ?2
         WHERE user_id = ?3 AND credential_id = ?4",
    )
    .bind(&[
        (verified.sign_count as f64).into(),
        now.into(),
        verified.user_id.clone().into(),
        verified.credential_id.clone().into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    webauthn::consume_challenge(db, &challenge_row.id).await?;

    let user: Option<Value> = db
        .prepare("SELECT * FROM users WHERE id = ?1")
        .bind(&[verified.user_id.clone().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?;
    let Some(user) = user else {
        return Err(AppError::Unauthorized("Invalid user".to_string()));
    };
    let user = serde_json::from_value(user).map_err(|_| AppError::Internal)?;
    let prf_option = if verified.prf_status == PRF_STATUS_ENABLED {
        PrfOption::from_columns(
            verified.credential_id.clone(),
            verified.encrypted_private_key.as_deref(),
            verified.encrypted_user_key.as_deref(),
        )
    } else {
        None
    };

    Ok(PasskeyLoginResult { user, prf_option })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prf_status_reflects_capability_and_key_set() {
        assert_eq!(
            prepare_prf_keys(false, None, None, None).unwrap().0,
            PRF_STATUS_UNSUPPORTED
        );
        assert_eq!(
            prepare_prf_keys(true, None, None, None).unwrap().0,
            PRF_STATUS_SUPPORTED
        );
        assert_eq!(
            prepare_prf_keys(true, Some("user"), Some("public"), Some("private"))
                .unwrap()
                .0,
            PRF_STATUS_ENABLED
        );
    }

    #[test]
    fn prf_keys_must_be_all_or_none() {
        assert!(prepare_prf_keys(true, Some("user"), Some("public"), None).is_err());
        assert!(prepare_prf_keys(true, None, Some("public"), None).is_err());
        assert!(prepare_prf_keys(true, Some("user"), Some("public"), Some("private")).is_ok());
    }

    #[test]
    fn prf_keys_reject_oversized_values() {
        let oversized = "a".repeat(MAX_ENCRYPTED_KEY_LEN + 1);
        assert!(prepare_prf_keys(true, Some(&oversized), Some("public"), Some("private")).is_err());
    }

    #[test]
    fn prf_option_uses_identity_and_sync_casing() {
        let option = PrfOption::from_columns("credential", Some("private"), Some("user")).unwrap();
        let identity = option.identity_json();
        assert_eq!(identity["EncryptedPrivateKey"], "private");
        assert_eq!(identity["EncryptedUserKey"], "user");
        assert_eq!(identity["CredentialId"], "credential");

        let sync = option.sync_json();
        assert_eq!(sync["encryptedPrivateKey"], "private");
        assert_eq!(sync["encryptedUserKey"], "user");
        assert_eq!(sync["credentialId"], "credential");
    }
}
