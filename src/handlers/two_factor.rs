use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use chrono::Utc;
use constant_time_eq::constant_time_eq;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};
use worker::Env;

use crate::auth::Claims;
use crate::db;
use crate::error::AppError;
use crate::two_factor;
use crate::webauthn;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableAuthenticatorRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisableAuthenticatorRequest {
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordOrOtpData {
    #[serde(alias = "MasterPasswordHash")]
    pub master_password_hash: Option<String>,
    pub otp: Option<String>,
}

impl PasswordOrOtpData {
    pub async fn validate(
        &self,
        db: &worker::D1Database,
        user_id: &str,
        env: &Env,
    ) -> Result<(), AppError> {
        match (&self.master_password_hash, &self.otp) {
            (Some(master_password_hash), None) => {
                let stored_hash: Option<String> = db
                    .prepare("SELECT master_password_hash FROM users WHERE id = ?1")
                    .bind(&[user_id.into()])?
                    .first(Some("master_password_hash"))
                    .await
                    .map_err(|_| AppError::Database)?;
                let Some(stored_hash) = stored_hash else {
                    return Err(AppError::NotFound("User not found".to_string()));
                };
                if !constant_time_eq(stored_hash.as_bytes(), master_password_hash.as_bytes()) {
                    return Err(AppError::Unauthorized("Invalid credentials".to_string()));
                }
                Ok(())
            }
            (None, Some(otp)) => {
                let secret_enc: Option<String> = db
                    .prepare(
                        "SELECT secret_enc FROM two_factor_authenticator WHERE user_id = ?1 AND enabled = 1",
                    )
                    .bind(&[user_id.into()])?
                    .first(Some("secret_enc"))
                    .await
                    .map_err(|_| AppError::Database)?;
                let Some(secret_enc) = secret_enc else {
                    return Err(AppError::Unauthorized(
                        "Authenticator is not enabled".to_string(),
                    ));
                };
                let enc_key = env.secret("TWO_FACTOR_ENC_KEY").ok().map(|s| s.to_string());
                let secret = two_factor::decrypt_secret_with_optional_key(
                    enc_key.as_deref(),
                    user_id,
                    &secret_enc,
                )?;
                if !two_factor::verify_totp_code(&secret, otp)? {
                    return Err(AppError::Unauthorized("Invalid OTP".to_string()));
                }
                Ok(())
            }
            _ => Err(AppError::BadRequest("No validation provided".to_string())),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(i64),
    String(String),
}

impl NumberOrString {
    fn into_string(self) -> String {
        match self {
            NumberOrString::Number(n) => n.to_string(),
            NumberOrString::String(s) => s,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnableAuthenticatorData {
    key: String,
    token: NumberOrString,
    master_password_hash: Option<String>,
    otp: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisableAuthenticatorData {
    key: String,
    master_password_hash: String,
    #[serde(rename = "type")]
    r#type: NumberOrString,
}

#[worker::send]
pub async fn two_factor_status(
    claims: Claims,
    State(env): State<Arc<Env>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    let totp_enabled = two_factor::is_authenticator_enabled(&db, &claims.sub).await?;
    let webauthn_enabled = two_factor::is_webauthn_enabled(&db, &claims.sub).await?;
    let mut providers = Vec::new();
    if totp_enabled {
        providers.push(two_factor::TWO_FACTOR_PROVIDER_AUTHENTICATOR);
    }
    if webauthn_enabled {
        providers.push(two_factor::TWO_FACTOR_PROVIDER_WEBAUTHN);
    }
    Ok(Json(json!({
        "enabled": totp_enabled || webauthn_enabled,
        "providers": providers
    })))
}

#[worker::send]
pub async fn authenticator_request(
    claims: Claims,
    State(env): State<Arc<Env>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    let now = Utc::now().to_rfc3339();

    let user_email: Option<String> = db
        .prepare("SELECT email FROM users WHERE id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(Some("email"))
        .await
        .map_err(|_| AppError::Database)?;
    let user_email = user_email.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let secret_encoded = two_factor::generate_totp_secret_base32_20();
    let two_factor_key_b64 = env.secret("TWO_FACTOR_ENC_KEY").ok().map(|s| s.to_string());
    let secret_enc = two_factor::encrypt_secret_with_optional_key(
        two_factor_key_b64.as_deref(),
        &claims.sub,
        &secret_encoded,
    )?;

    two_factor::upsert_authenticator_secret(&db, &claims.sub, secret_enc, false, &now).await?;

    let issuer = env
        .var("TWO_FACTOR_ISSUER")
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "Warden Worker".to_string());
    let issuer = issuer.replace(':', "");
    let account = user_email.replace(':', "");
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret_encoded.clone())
            .to_bytes()
            .map_err(|_| AppError::Internal)?,
        Some(issuer.clone()),
        account.clone(),
    )
    .map_err(|_| AppError::Internal)?;
    let otpauth = totp.get_url();
    let qr_base64 = totp.get_qr_base64().map_err(|_| AppError::Internal)?;

    Ok(Json(json!({
        "secret": secret_encoded,
        "otpauth": otpauth,
        "qrBase64": qr_base64
    })))
}

#[worker::send]
pub async fn get_authenticator(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<PasswordOrOtpData>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    payload.validate(&db, &claims.sub, &env).await?;

    let enabled = two_factor::is_authenticator_enabled(&db, &claims.sub).await?;
    let key = if enabled {
        let secret_enc = two_factor::get_authenticator_secret_enc(&db, &claims.sub)
            .await?
            .ok_or_else(|| AppError::Internal)?;
        let two_factor_key_b64 = env.secret("TWO_FACTOR_ENC_KEY").ok().map(|s| s.to_string());
        two_factor::decrypt_secret_with_optional_key(
            two_factor_key_b64.as_deref(),
            &claims.sub,
            &secret_enc,
        )?
    } else {
        two_factor::generate_totp_secret_base32_20()
    };

    Ok(Json(json!({
        "enabled": enabled,
        "key": key,
        "userVerificationToken": null,
        "object": "twoFactorAuthenticator"
    })))
}

#[worker::send]
pub async fn get_recovery(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<PasswordOrOtpData>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    payload.validate(&db, &claims.sub, &env).await?;
    let code = match two_factor::get_recovery_code(&db, &claims.sub).await? {
        Some(code) => code,
        None => {
            let code = two_factor::generate_recovery_code();
            db.prepare("UPDATE users SET totp_recover = ?1, updated_at = ?2 WHERE id = ?3")
                .bind(&[
                    code.clone().into(),
                    Utc::now().to_rfc3339().into(),
                    claims.sub.clone().into(),
                ])?
                .run()
                .await
                .map_err(|_| AppError::Database)?;
            two_factor::revoke_device_remember_tokens(&db, &claims.sub).await?;
            code
        }
    };
    Ok(Json(json!({ "code": code, "object": "twoFactorRecover" })))
}

#[worker::send]
pub async fn activate_authenticator(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<EnableAuthenticatorData>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;

    PasswordOrOtpData {
        master_password_hash: payload.master_password_hash.clone(),
        otp: payload.otp.clone(),
    }
    .validate(&db, &claims.sub, &env)
    .await?;

    let key = payload.key.trim().to_uppercase();
    let key_bytes = Secret::Encoded(key.clone())
        .to_bytes()
        .map_err(|_| AppError::BadRequest("Invalid totp secret".to_string()))?;
    if key_bytes.len() != 20 {
        return Err(AppError::BadRequest("Invalid key length".to_string()));
    }

    let token = payload.token.into_string();
    if !two_factor::verify_totp_code(&key, &token)? {
        return Err(AppError::BadRequest("Invalid TOTP code".to_string()));
    }

    let now = Utc::now().to_rfc3339();
    two_factor::ensure_recovery_code_column(&db).await?;
    let recovery_code = two_factor::get_recovery_code(&db, &claims.sub)
        .await?
        .unwrap_or_else(two_factor::generate_recovery_code);
    let two_factor_key_b64 = env.secret("TWO_FACTOR_ENC_KEY").ok().map(|s| s.to_string());
    let secret_enc = two_factor::encrypt_secret_with_optional_key(
        two_factor_key_b64.as_deref(),
        &claims.sub,
        &key,
    )?;
    two_factor::upsert_authenticator_secret(&db, &claims.sub, secret_enc, true, &now).await?;
    db.prepare("UPDATE users SET totp_recover = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(&[
            recovery_code.clone().into(),
            now.clone().into(),
            claims.sub.clone().into(),
        ])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    two_factor::revoke_device_remember_tokens(&db, &claims.sub).await?;

    Ok(Json(json!({
        "enabled": true,
        "key": key,
        "recoveryCode": recovery_code,
        "userVerificationToken": null,
        "object": "twoFactorAuthenticator"
    })))
}

#[worker::send]
pub async fn activate_authenticator_put(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<EnableAuthenticatorData>,
) -> Result<Json<serde_json::Value>, AppError> {
    activate_authenticator(claims, State(env), Json(payload)).await
}

#[worker::send]
pub async fn disable_authenticator_vw(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<DisableAuthenticatorData>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;

    let stored_hash: Option<String> = db
        .prepare("SELECT master_password_hash FROM users WHERE id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(Some("master_password_hash"))
        .await
        .map_err(|_| AppError::Database)?;
    let Some(stored_hash) = stored_hash else {
        return Err(AppError::NotFound("User not found".to_string()));
    };
    if !constant_time_eq(
        stored_hash.as_bytes(),
        payload.master_password_hash.as_bytes(),
    ) {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    if let Some(secret_enc) = two_factor::get_authenticator_secret_enc(&db, &claims.sub).await? {
        let two_factor_key_b64 = env.secret("TWO_FACTOR_ENC_KEY").ok().map(|s| s.to_string());
        let secret_encoded = two_factor::decrypt_secret_with_optional_key(
            two_factor_key_b64.as_deref(),
            &claims.sub,
            &secret_enc,
        )?;
        if secret_encoded.eq_ignore_ascii_case(payload.key.trim()) {
            two_factor::disable_authenticator(&db, &claims.sub).await?;
            two_factor::clear_recovery_code(&db, &claims.sub).await?;
            two_factor::revoke_device_remember_tokens(&db, &claims.sub).await?;
        } else {
            return Err(AppError::BadRequest(
                "TOTP key does not match recorded value".to_string(),
            ));
        }
    }

    let type_ = match payload.r#type {
        NumberOrString::Number(n) => n as i32,
        NumberOrString::String(s) => s
            .parse::<i32>()
            .unwrap_or(two_factor::TWO_FACTOR_PROVIDER_AUTHENTICATOR),
    };

    Ok(Json(json!({
        "enabled": false,
        "key": payload.key,
        "userVerificationToken": null,
        "type": type_,
        "object": "twoFactorAuthenticator"
    })))
}

#[worker::send]
pub async fn authenticator_enable(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<EnableAuthenticatorRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    let now = Utc::now().to_rfc3339();

    let secret_enc = two_factor::get_authenticator_secret_enc(&db, &claims.sub)
        .await?
        .ok_or_else(|| AppError::BadRequest("No pending authenticator setup".to_string()))?;
    let two_factor_key_b64 = env.secret("TWO_FACTOR_ENC_KEY").ok().map(|s| s.to_string());
    let secret_encoded = match two_factor::decrypt_secret_with_optional_key(
        two_factor_key_b64.as_deref(),
        &claims.sub,
        &secret_enc,
    ) {
        Ok(v) => v,
        Err(e) => {
            let _ = two_factor::disable_authenticator(&db, &claims.sub).await;
            return Err(e);
        }
    };
    if !two_factor::verify_totp_code(&secret_encoded, &payload.code)? {
        return Err(AppError::BadRequest("Invalid TOTP code".to_string()));
    }

    two_factor::upsert_authenticator_secret(&db, &claims.sub, secret_enc, true, &now).await?;
    two_factor::revoke_device_remember_tokens(&db, &claims.sub).await?;
    Ok(Json(json!({})))
}

#[worker::send]
pub async fn authenticator_disable(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<DisableAuthenticatorRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    let secret_enc = two_factor::get_authenticator_secret_enc(&db, &claims.sub)
        .await?
        .ok_or_else(|| AppError::BadRequest("Authenticator not enabled".to_string()))?;
    let two_factor_key_b64 = env.secret("TWO_FACTOR_ENC_KEY").ok().map(|s| s.to_string());
    let secret_encoded = two_factor::decrypt_secret_with_optional_key(
        two_factor_key_b64.as_deref(),
        &claims.sub,
        &secret_enc,
    )?;
    if !two_factor::verify_totp_code(&secret_encoded, &payload.code)? {
        return Err(AppError::BadRequest("Invalid TOTP code".to_string()));
    }

    two_factor::disable_authenticator(&db, &claims.sub).await?;
    two_factor::clear_recovery_code(&db, &claims.sub).await?;
    two_factor::revoke_device_remember_tokens(&db, &claims.sub).await?;
    Ok(Json(json!({})))
}

async fn list_webauthn_keys(
    db: &worker::D1Database,
    user_id: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    webauthn::ensure_tables(db).await?;
    let rows = db
        .prepare("SELECT key_id, name FROM two_factor_webauthn WHERE user_id = ?1 ORDER BY key_id")
        .bind(&[user_id.into()])?
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results::<serde_json::Value>()
        .map_err(|_| AppError::Database)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "Id": row.get("key_id").cloned().unwrap_or(serde_json::Value::Null),
                "Name": row.get("name").cloned().unwrap_or(serde_json::Value::Null),
                "Migrated": false
            })
        })
        .collect())
}

#[worker::send]
pub async fn get_webauthn(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<PasswordOrOtpData>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    payload.validate(&db, &claims.sub, &env).await?;
    let keys = list_webauthn_keys(&db, &claims.sub).await?;
    Ok(Json(json!({
        "Enabled": !keys.is_empty(),
        "Keys": keys
    })))
}

#[worker::send]
pub async fn get_webauthn_challenge(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Json(payload): Json<PasswordOrOtpData>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    payload.validate(&db, &claims.sub, &env).await?;
    webauthn::ensure_tables(&db).await?;
    let rp = webauthn::rp_config(&env, &headers)?;
    let _ = webauthn::cleanup_expired_challenges(&db).await;

    let user: Option<serde_json::Value> = db
        .prepare("SELECT email, name FROM users WHERE id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?;
    let user = user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let email = user
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let display_name = user
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .unwrap_or(&email)
        .to_string();

    let rows = db
        .prepare("SELECT credential_id FROM two_factor_webauthn WHERE user_id = ?1")
        .bind(&[claims.sub.clone().into()])?
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results::<serde_json::Value>()
        .map_err(|_| AppError::Database)?;
    let exclude: Vec<serde_json::Value> = rows
        .into_iter()
        .filter_map(|row| {
            row.get("credential_id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .map(|id| json!({ "type": "public-key", "id": id }))
        .collect();

    let challenge = webauthn::random_challenge();
    webauthn::store_challenge(
        &db,
        Some(&claims.sub),
        "2fa_register",
        &challenge,
        None,
        webauthn::CHALLENGE_TTL_REGISTER_SECS,
    )
    .await?;

    Ok(Json(json!({
        "attestation": "none",
        "authenticatorSelection": {
            "residentKey": "preferred",
            "requireResidentKey": false,
            "userVerification": "required"
        },
        "challenge": challenge,
        "excludeCredentials": exclude,
        "extensions": {},
        "pubKeyCredParams": [{ "type": "public-key", "alg": -7 }],
        "rp": { "id": rp.rp_id, "name": rp.rp_name },
        "timeout": 60000,
        "user": {
            "id": webauthn::b64url_encode(claims.sub.as_bytes()),
            "name": email,
            "displayName": display_name
        }
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorWebAuthnPayload {
    pub id: NumberOrString,
    pub name: Option<String>,
    pub device_response: Option<TwoFactorAttestation>,
    #[serde(flatten)]
    pub verification: PasswordOrOtpData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorAttestation {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub response: TwoFactorAttestationResponse,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoFactorAttestationResponse {
    #[serde(default, alias = "AttestationObject")]
    pub attestation_object: String,
    #[serde(default, alias = "clientDataJson", alias = "clientDataJSON")]
    pub client_data_json: String,
}

#[worker::send]
pub async fn activate_webauthn(
    claims: Claims,
    State(env): State<Arc<Env>>,
    headers: HeaderMap,
    Json(payload): Json<TwoFactorWebAuthnPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    payload
        .verification
        .validate(&db, &claims.sub, &env)
        .await?;
    webauthn::ensure_tables(&db).await?;
    let rp = webauthn::rp_config(&env, &headers)?;

    let key_id: i32 = payload
        .id
        .into_string()
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid key id".to_string()))?;
    if !(1..=5).contains(&key_id) {
        return Err(AppError::BadRequest("Invalid key id".to_string()));
    }
    let attestation = payload
        .device_response
        .ok_or_else(|| AppError::BadRequest("Missing attestation".to_string()))?;

    let client_data_json = webauthn::b64_decode(&attestation.response.client_data_json)
        .map_err(|_| AppError::BadRequest("Invalid client data".to_string()))?;
    let client_data = webauthn::parse_client_data(&client_data_json)?;
    if client_data.typ != "webauthn.create" {
        return Err(AppError::BadRequest(
            "Unexpected WebAuthn ceremony type".to_string(),
        ));
    }
    let challenge_row =
        webauthn::load_challenge_for_user(&db, &claims.sub, "2fa_register", &client_data.challenge)
            .await?
            .ok_or_else(|| AppError::BadRequest("Invalid or expired challenge".to_string()))?;
    webauthn::validate_client_data(
        &client_data,
        "webauthn.create",
        &challenge_row.challenge,
        &rp.origin,
    )?;

    let attestation_bytes = webauthn::b64_decode(&attestation.response.attestation_object)
        .map_err(|_| AppError::BadRequest("Invalid attestation object".to_string()))?;
    let parsed = webauthn::parse_attestation_object(&attestation_bytes)?;
    let auth_data = webauthn::parse_auth_data(&parsed.auth_data, true)?;
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
    webauthn::verify_attestation(&parsed, &client_data_hash, credential)?;

    let credential_id = webauthn::b64url_encode(&credential.credential_id);
    if let Some(device_id) = attestation.id.as_deref() {
        if webauthn::normalize_credential_id(device_id)? != credential_id {
            return Err(AppError::BadRequest(
                "Credential id does not match attestation".to_string(),
            ));
        }
    }
    let duplicate: Option<i64> = db
        .prepare("SELECT 1 AS ok FROM two_factor_webauthn WHERE credential_id = ?1")
        .bind(&[credential_id.clone().into()])?
        .first(Some("ok"))
        .await
        .map_err(|_| AppError::Database)?;
    if duplicate.is_some() {
        return Err(AppError::BadRequest(
            "This security key is already registered".to_string(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("Security key {key_id}"));
    db.prepare(
        "INSERT INTO two_factor_webauthn
            (user_id, key_id, name, credential_id, public_key, sign_count, created_at, updated_at, last_used_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)
         ON CONFLICT(user_id, key_id) DO UPDATE SET
            name = excluded.name,
            credential_id = excluded.credential_id,
            public_key = excluded.public_key,
            sign_count = excluded.sign_count,
            updated_at = excluded.updated_at",
    )
    .bind(&[
        claims.sub.clone().into(),
        (key_id as i64).into(),
        name.into(),
        credential_id.into(),
        webauthn::b64_std_encode(&credential.public_key).into(),
        (auth_data.sign_count as i64).into(),
        now.into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    webauthn::consume_challenge(&db, &challenge_row.id).await?;
    two_factor::revoke_device_remember_tokens(&db, &claims.sub).await?;

    let keys = list_webauthn_keys(&db, &claims.sub).await?;
    Ok(Json(json!({ "Enabled": true, "Keys": keys })))
}

#[worker::send]
pub async fn deactivate_webauthn(
    claims: Claims,
    State(env): State<Arc<Env>>,
    Json(payload): Json<TwoFactorWebAuthnPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = db::get_db(&env)?;
    payload
        .verification
        .validate(&db, &claims.sub, &env)
        .await?;
    webauthn::ensure_tables(&db).await?;

    let key_id: i32 = payload
        .id
        .into_string()
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid key id".to_string()))?;
    db.prepare("DELETE FROM two_factor_webauthn WHERE user_id = ?1 AND key_id = ?2")
        .bind(&[claims.sub.clone().into(), (key_id as i64).into()])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    two_factor::revoke_device_remember_tokens(&db, &claims.sub).await?;

    let keys = list_webauthn_keys(&db, &claims.sub).await?;
    Ok(Json(json!({
        "Enabled": !keys.is_empty(),
        "Keys": keys
    })))
}

pub async fn webauthn_login_options(
    db: &worker::D1Database,
    env: &Env,
    headers: &HeaderMap,
    user_id: &str,
) -> Result<Option<serde_json::Value>, AppError> {
    webauthn::ensure_tables(db).await?;
    let rows = db
        .prepare("SELECT credential_id FROM two_factor_webauthn WHERE user_id = ?1 ORDER BY key_id")
        .bind(&[user_id.into()])?
        .all()
        .await
        .map_err(|_| AppError::Database)?
        .results::<serde_json::Value>()
        .map_err(|_| AppError::Database)?;
    if rows.is_empty() {
        return Ok(None);
    }
    let rp = webauthn::rp_config(env, headers)?;
    let _ = webauthn::cleanup_expired_challenges(db).await;
    let challenge = webauthn::random_challenge();
    webauthn::store_challenge(
        db,
        Some(user_id),
        "2fa_login",
        &challenge,
        None,
        webauthn::CHALLENGE_TTL_LOGIN_SECS,
    )
    .await?;
    let allow_credentials: Vec<serde_json::Value> = rows
        .into_iter()
        .filter_map(|row| {
            row.get("credential_id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .map(|id| json!({ "type": "public-key", "id": id }))
        .collect();
    Ok(Some(json!({
        "challenge": challenge,
        "allowCredentials": allow_credentials,
        "rpId": rp.rp_id,
        "timeout": 60000,
        "userVerification": "preferred"
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TwoFactorAssertion {
    id: Option<String>,
    #[serde(default)]
    response: TwoFactorAssertionResponse,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TwoFactorAssertionResponse {
    #[serde(default)]
    authenticator_data: String,
    #[serde(default)]
    signature: String,
    #[serde(default, alias = "clientDataJSON")]
    client_data_json: String,
}

pub async fn verify_webauthn_2fa(
    db: &worker::D1Database,
    env: &Env,
    headers: &HeaderMap,
    user_id: &str,
    token: &str,
) -> Result<bool, AppError> {
    webauthn::ensure_tables(db).await?;
    let rp = webauthn::rp_config(env, headers)?;

    let Ok(assertion) = serde_json::from_str::<TwoFactorAssertion>(token) else {
        return Ok(false);
    };
    let Ok(client_data_json) = webauthn::b64_decode(&assertion.response.client_data_json) else {
        return Ok(false);
    };
    let Ok(client_data) = webauthn::parse_client_data(&client_data_json) else {
        return Ok(false);
    };
    let Some(challenge_row) =
        webauthn::load_challenge_for_user(db, user_id, "2fa_login", &client_data.challenge).await?
    else {
        return Ok(false);
    };
    if webauthn::validate_client_data(
        &client_data,
        "webauthn.get",
        &challenge_row.challenge,
        &rp.origin,
    )
    .is_err()
    {
        return Ok(false);
    }

    let Ok(auth_data_bytes) = webauthn::b64_decode(&assertion.response.authenticator_data) else {
        return Ok(false);
    };
    let Ok(auth_data) = webauthn::parse_auth_data(&auth_data_bytes, false) else {
        return Ok(false);
    };
    if webauthn::validate_rp_id_hash(&auth_data, &rp.rp_id).is_err()
        || webauthn::validate_user_presence(&auth_data, false).is_err()
    {
        return Ok(false);
    }

    let Some(credential_id) = assertion
        .id
        .as_deref()
        .and_then(|id| webauthn::normalize_credential_id(id).ok())
    else {
        return Ok(false);
    };
    let row: Option<serde_json::Value> = db
        .prepare(
            "SELECT public_key, sign_count FROM two_factor_webauthn
             WHERE user_id = ?1 AND credential_id = ?2",
        )
        .bind(&[user_id.into(), credential_id.clone().into()])?
        .first(None)
        .await
        .map_err(|_| AppError::Database)?;
    let Some(row) = row else {
        return Ok(false);
    };
    let Some(public_key) = row.get("public_key").and_then(|v| v.as_str()) else {
        return Ok(false);
    };
    let stored_count = row.get("sign_count").and_then(|v| v.as_i64()).unwrap_or(0);
    let Ok(public_key) = webauthn::b64_decode(public_key) else {
        return Ok(false);
    };
    let Ok(signature) = webauthn::b64_decode(&assertion.response.signature) else {
        return Ok(false);
    };
    if webauthn::verify_assertion_signature(
        &public_key,
        &auth_data_bytes,
        &client_data_json,
        &signature,
    )
    .is_err()
    {
        return Ok(false);
    }

    let received = auth_data.sign_count as i64;
    if received != 0 && stored_count != 0 && received <= stored_count {
        return Ok(false);
    }
    let now = Utc::now().to_rfc3339();
    db.prepare(
        "UPDATE two_factor_webauthn
         SET sign_count = ?1, last_used_at = ?2, updated_at = ?2
         WHERE user_id = ?3 AND credential_id = ?4",
    )
    .bind(&[
        received.max(stored_count).into(),
        now.into(),
        user_id.into(),
        credential_id.into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    webauthn::consume_challenge(db, &challenge_row.id).await?;
    Ok(true)
}
