use std::io::Cursor;

use axum::http::HeaderMap;
use base64::{engine::general_purpose, Engine as _};
use ciborium::value::Value as CborValue;
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use wasm_bindgen::JsValue;
use worker::{D1Database, Env};

use crate::error::AppError;

pub const RP_NAME: &str = "Warden Worker";
pub const CHALLENGE_TTL_LOGIN_SECS: i64 = 5 * 60;
pub const CHALLENGE_TTL_REGISTER_SECS: i64 = 10 * 60;
pub const FLAG_UP: u8 = 0x01;
pub const FLAG_UV: u8 = 0x04;
pub const FLAG_AT: u8 = 0x40;

pub struct RpConfig {
    pub rp_id: String,
    pub origin: String,
    pub rp_name: String,
}

pub struct AuthData {
    pub rp_id_hash: [u8; 32],
    pub flags: u8,
    pub sign_count: u32,
    pub credential: Option<AttestedCredential>,
}

pub struct AttestedCredential {
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
}

pub struct ClientData {
    pub typ: String,
    pub challenge: String,
    pub origin: String,
    pub cross_origin: bool,
}

pub struct ParsedAttestation {
    pub fmt: String,
    pub auth_data: Vec<u8>,
    pub att_stmt: Option<CborValue>,
}

fn bad(message: &str) -> AppError {
    AppError::BadRequest(message.to_string())
}

fn invalid(message: &str) -> AppError {
    AppError::Unauthorized(message.to_string())
}

pub fn b64_decode(input: &str) -> Result<Vec<u8>, AppError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(bad("Empty base64 value"));
    }
    for engine in [
        &general_purpose::URL_SAFE_NO_PAD,
        &general_purpose::URL_SAFE,
        &general_purpose::STANDARD_NO_PAD,
        &general_purpose::STANDARD,
    ] {
        if let Ok(bytes) = engine.decode(text) {
            return Ok(bytes);
        }
    }
    Err(bad("Invalid base64 value"))
}

pub fn b64url_encode(bytes: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn b64_std_encode(bytes: &[u8]) -> String {
    general_purpose::STANDARD.encode(bytes)
}

pub fn normalize_credential_id(raw: &str) -> Result<String, AppError> {
    Ok(b64url_encode(&b64_decode(raw)?))
}

pub fn random_challenge() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    b64url_encode(&bytes)
}

pub fn random_token() -> String {
    random_challenge()
}

pub fn sha256_hex(input: &str) -> String {
    hex::encode(Sha256::digest(input.as_bytes()))
}

pub fn parse_client_data(bytes: &[u8]) -> Result<ClientData, AppError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| bad("Invalid client data"))?;
    let typ = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("Invalid client data type"))?;
    let challenge = value
        .get("challenge")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("Invalid client data challenge"))?;
    let origin = value
        .get("origin")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("Invalid client data origin"))?;
    Ok(ClientData {
        typ: typ.to_string(),
        challenge: challenge.to_string(),
        origin: origin.to_string(),
        cross_origin: value
            .get("crossOrigin")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub fn validate_client_data(
    client_data: &ClientData,
    expected_type: &str,
    challenge: &str,
    origin: &str,
) -> Result<(), AppError> {
    if client_data.typ != expected_type {
        return Err(invalid("Unexpected WebAuthn ceremony type"));
    }
    if client_data.challenge != challenge {
        return Err(invalid("WebAuthn challenge mismatch"));
    }
    if client_data.cross_origin {
        return Err(invalid("Cross-origin WebAuthn is not allowed"));
    }
    let expected = normalize_origin(origin)?;
    let actual = normalize_origin(&client_data.origin)?;
    if actual != expected {
        return Err(invalid("WebAuthn origin mismatch"));
    }
    Ok(())
}

pub fn parse_auth_data(bytes: &[u8], require_attested: bool) -> Result<AuthData, AppError> {
    if bytes.len() < 37 {
        return Err(invalid("Invalid authenticator data"));
    }
    let mut rp_id_hash = [0u8; 32];
    rp_id_hash.copy_from_slice(&bytes[..32]);
    let flags = bytes[32];
    let sign_count = u32::from_be_bytes([bytes[33], bytes[34], bytes[35], bytes[36]]);
    let mut credential = None;

    if flags & FLAG_AT != 0 {
        if bytes.len() < 55 {
            return Err(invalid("Invalid attested credential data"));
        }
        let credential_id_len = u16::from_be_bytes([bytes[53], bytes[54]]) as usize;
        let credential_start = 55usize;
        let credential_end = credential_start
            .checked_add(credential_id_len)
            .ok_or_else(|| invalid("Invalid credential id length"))?;
        if credential_id_len == 0 || bytes.len() < credential_end {
            return Err(invalid("Invalid credential id"));
        }
        let credential_id = bytes[credential_start..credential_end].to_vec();

        let mut cursor = Cursor::new(&bytes[credential_end..]);
        let cose_value: CborValue =
            ciborium::de::from_reader(&mut cursor).map_err(|_| invalid("Invalid COSE key"))?;
        let consumed = cursor.position() as usize;
        if consumed == 0 || credential_end + consumed > bytes.len() {
            return Err(invalid("Invalid COSE key length"));
        }
        let public_key = bytes[credential_end..credential_end + consumed].to_vec();
        // Parse once so malformed keys are rejected before they are stored.
        parse_cose_value(&cose_value)?;
        credential = Some(AttestedCredential {
            credential_id,
            public_key,
        });
    } else if require_attested {
        return Err(invalid("Missing attested credential data"));
    }

    Ok(AuthData {
        rp_id_hash,
        flags,
        sign_count,
        credential,
    })
}

pub fn parse_attestation_object(bytes: &[u8]) -> Result<ParsedAttestation, AppError> {
    let value: CborValue =
        ciborium::de::from_reader(bytes).map_err(|_| invalid("Invalid attestation object"))?;
    let map = value
        .as_map()
        .ok_or_else(|| invalid("Invalid attestation object"))?;
    let fmt = map_text(map, "fmt").ok_or_else(|| invalid("Missing attestation format"))?;
    let auth_data = map_bytes(map, "authData")
        .ok_or_else(|| invalid("Missing authenticator data"))?
        .to_vec();
    Ok(ParsedAttestation {
        fmt,
        auth_data,
        att_stmt: map_get(map, "attStmt").cloned(),
    })
}

pub fn verify_attestation(
    attestation: &ParsedAttestation,
    client_data_hash: &[u8],
    credential: &AttestedCredential,
) -> Result<(), AppError> {
    match attestation.fmt.as_str() {
        "none" => Ok(()),
        "packed" => {
            let statement = attestation
                .att_stmt
                .as_ref()
                .and_then(CborValue::as_map)
                .ok_or_else(|| invalid("Invalid packed attestation"))?;
            if map_get_text(statement, "x5c").is_some() {
                return Err(invalid("Attestation certificates are not supported"));
            }
            let alg = map_text_int(statement, "alg")
                .ok_or_else(|| invalid("Missing packed attestation algorithm"))?;
            if alg != -7 {
                return Err(invalid("Only ES256 attestation is supported"));
            }
            let signature = map_bytes(statement, "sig")
                .ok_or_else(|| invalid("Missing packed attestation signature"))?;
            let mut message = Vec::with_capacity(attestation.auth_data.len() + 32);
            message.extend_from_slice(&attestation.auth_data);
            message.extend_from_slice(client_data_hash);
            verify_es256(&credential.public_key, &message, signature)
        }
        _ => Err(invalid("Unsupported attestation format")),
    }
}

pub fn verify_assertion_signature(
    public_key: &[u8],
    auth_data: &[u8],
    client_data_json: &[u8],
    signature: &[u8],
) -> Result<(), AppError> {
    let mut message = Vec::with_capacity(auth_data.len() + 32);
    message.extend_from_slice(auth_data);
    message.extend_from_slice(&Sha256::digest(client_data_json));
    verify_es256(public_key, &message, signature)
}

fn verify_es256(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), AppError> {
    let cose_value: CborValue =
        ciborium::de::from_reader(public_key).map_err(|_| invalid("Invalid COSE key"))?;
    let key = parse_cose_value(&cose_value)?;
    let signature = Signature::from_der(signature)
        .or_else(|_| Signature::from_slice(signature))
        .map_err(|_| invalid("Invalid WebAuthn signature encoding"))?;
    key.verify(message, &signature)
        .map_err(|_| invalid("Invalid WebAuthn signature"))
}

fn parse_cose_value(value: &CborValue) -> Result<VerifyingKey, AppError> {
    let map = value.as_map().ok_or_else(|| invalid("Invalid COSE key"))?;
    let kty = map_int(map, 1).ok_or_else(|| invalid("Missing COSE key type"))?;
    let alg = map_int(map, 3).ok_or_else(|| invalid("Missing COSE algorithm"))?;
    let crv = map_int(map, -1).ok_or_else(|| invalid("Missing COSE curve"))?;
    if kty != 2 || alg != -7 || crv != 1 {
        return Err(invalid("Only ES256 P-256 credentials are supported"));
    }
    let x = map_bytes_int(map, -2).ok_or_else(|| invalid("Missing COSE x coordinate"))?;
    let y = map_bytes_int(map, -3).ok_or_else(|| invalid("Missing COSE y coordinate"))?;
    if x.len() != 32 || y.len() != 32 {
        return Err(invalid("Invalid P-256 coordinate length"));
    }
    let mut sec1 = Vec::with_capacity(65);
    sec1.push(0x04);
    sec1.extend_from_slice(x);
    sec1.extend_from_slice(y);
    VerifyingKey::from_sec1_bytes(&sec1).map_err(|_| invalid("Invalid P-256 public key"))
}

pub fn validate_rp_id_hash(auth_data: &AuthData, rp_id: &str) -> Result<(), AppError> {
    let expected = Sha256::digest(rp_id.as_bytes());
    if auth_data.rp_id_hash[..] != expected[..] {
        return Err(invalid("WebAuthn RP ID mismatch"));
    }
    Ok(())
}

pub fn validate_user_presence(auth_data: &AuthData, require_uv: bool) -> Result<(), AppError> {
    if auth_data.flags & FLAG_UP == 0 {
        return Err(invalid("WebAuthn user presence is required"));
    }
    if require_uv && auth_data.flags & FLAG_UV == 0 {
        return Err(invalid("WebAuthn user verification is required"));
    }
    Ok(())
}

fn env_value(env: &Env, name: &str) -> Option<String> {
    env.secret(name)
        .ok()
        .map(|value| value.to_string())
        .or_else(|| env.var(name).ok().map(|value| value.to_string()))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn rp_config(env: &Env, headers: &HeaderMap) -> Result<RpConfig, AppError> {
    let explicit_origin = env_value(env, "WEBAUTHN_ORIGIN");
    let host = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost");
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .unwrap_or("https");
    let origin = explicit_origin.unwrap_or_else(|| format!("{proto}://{host}"));
    let parts = parse_origin(&origin)?;

    let rp_id = env_value(env, "WEBAUTHN_RP_ID").unwrap_or_else(|| parts.host.clone());
    let rp_id = rp_id.trim().trim_end_matches('.').to_ascii_lowercase();
    if rp_id.is_empty() || rp_id.contains('/') || rp_id.contains(':') {
        return Err(bad("Invalid WEBAUTHN_RP_ID"));
    }
    let origin_host = parts
        .host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    if origin_host != rp_id && !origin_host.ends_with(&format!(".{rp_id}")) {
        return Err(bad("WEBAUTHN_RP_ID does not match the request origin"));
    }

    Ok(RpConfig {
        rp_id,
        origin: parts.normalized(),
        rp_name: RP_NAME.to_string(),
    })
}

struct OriginParts {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl OriginParts {
    fn normalized(&self) -> String {
        let default_port = (self.scheme == "http" && self.port == Some(80))
            || (self.scheme == "https" && self.port == Some(443));
        match (self.port, default_port) {
            (Some(port), false) => format!("{}://{}:{}", self.scheme, self.host, port),
            _ => format!("{}://{}", self.scheme, self.host),
        }
    }
}

fn normalize_origin(origin: &str) -> Result<String, AppError> {
    Ok(parse_origin(origin)?.normalized())
}

fn parse_origin(origin: &str) -> Result<OriginParts, AppError> {
    let (scheme, rest) = origin
        .split_once("://")
        .ok_or_else(|| bad("Invalid WebAuthn origin"))?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(bad("Invalid WebAuthn origin scheme"));
    }
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| bad("Invalid WebAuthn origin"))?;
    let remainder = &rest[authority.len()..];
    if !remainder.is_empty() && remainder != "/" {
        return Err(bad("WebAuthn origin must not include a path"));
    }

    let (host, port) = if let Some(after) = authority.strip_prefix('[') {
        let (host, tail) = after
            .split_once(']')
            .ok_or_else(|| bad("Invalid WebAuthn origin"))?;
        let port = tail
            .strip_prefix(':')
            .filter(|value| !value.is_empty())
            .map(str::parse::<u16>)
            .transpose()
            .map_err(|_| bad("Invalid WebAuthn origin port"))?;
        (format!("[{}]", host.to_ascii_lowercase()), port)
    } else if let Some((host, port_text)) = authority.rsplit_once(':') {
        if !port_text.is_empty() && port_text.chars().all(|value| value.is_ascii_digit()) {
            (
                host.to_ascii_lowercase(),
                Some(
                    port_text
                        .parse::<u16>()
                        .map_err(|_| bad("Invalid WebAuthn origin port"))?,
                ),
            )
        } else {
            (authority.to_ascii_lowercase(), None)
        }
    } else {
        (authority.to_ascii_lowercase(), None)
    };

    if host.is_empty() {
        return Err(bad("Invalid WebAuthn origin host"));
    }
    Ok(OriginParts { scheme, host, port })
}

fn map_get<'a>(map: &'a [(CborValue, CborValue)], key: &str) -> Option<&'a CborValue> {
    map.iter().find_map(|(name, value)| match name {
        CborValue::Text(text) if text == key => Some(value),
        _ => None,
    })
}

fn map_text(map: &[(CborValue, CborValue)], key: &str) -> Option<String> {
    match map_get(map, key)? {
        CborValue::Text(text) => Some(text.clone()),
        _ => None,
    }
}

fn map_bytes<'a>(map: &'a [(CborValue, CborValue)], key: &str) -> Option<&'a [u8]> {
    match map_get(map, key)? {
        CborValue::Bytes(bytes) => Some(bytes.as_slice()),
        _ => None,
    }
}

fn map_int(map: &[(CborValue, CborValue)], key: i64) -> Option<i128> {
    map.iter().find_map(|(name, value)| {
        let name = match name {
            CborValue::Integer(number) => i128::try_from(*number).ok()?,
            _ => return None,
        };
        if name != key as i128 {
            return None;
        }
        match value {
            CborValue::Integer(number) => i128::try_from(*number).ok(),
            _ => None,
        }
    })
}

fn map_text_int(map: &[(CborValue, CborValue)], key: &str) -> Option<i128> {
    match map_get(map, key)? {
        CborValue::Integer(number) => i128::try_from(*number).ok(),
        _ => None,
    }
}

fn map_bytes_int<'a>(map: &'a [(CborValue, CborValue)], key: i64) -> Option<&'a [u8]> {
    map.iter().find_map(|(name, value)| {
        let name = match name {
            CborValue::Integer(number) => i128::try_from(*number).ok()?,
            _ => return None,
        };
        if name != key as i128 {
            return None;
        }
        match value {
            CborValue::Bytes(bytes) => Some(bytes.as_slice()),
            _ => None,
        }
    })
}

fn map_get_text<'a>(map: &'a [(CborValue, CborValue)], key: &str) -> Option<&'a CborValue> {
    map_get(map, key)
}

pub async fn ensure_tables(db: &D1Database) -> Result<(), AppError> {
    for sql in [
        "CREATE TABLE IF NOT EXISTS webauthn_credentials (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            credential_id TEXT NOT NULL UNIQUE,
            public_key TEXT NOT NULL,
            sign_count INTEGER NOT NULL DEFAULT 0,
            transports TEXT,
            prf_status INTEGER NOT NULL DEFAULT 2,
            encrypted_user_key TEXT,
            encrypted_public_key TEXT,
            encrypted_private_key TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_used_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        "CREATE INDEX IF NOT EXISTS idx_webauthn_credentials_user
            ON webauthn_credentials(user_id)",
        "CREATE TABLE IF NOT EXISTS two_factor_webauthn (
            user_id TEXT NOT NULL,
            key_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            credential_id TEXT NOT NULL UNIQUE,
            public_key TEXT NOT NULL,
            sign_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_used_at TEXT,
            PRIMARY KEY (user_id, key_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        "CREATE TABLE IF NOT EXISTS webauthn_challenges (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT,
            kind TEXT NOT NULL,
            challenge TEXT NOT NULL,
            token_hash TEXT,
            expires_at INTEGER NOT NULL,
            consumed_at INTEGER,
            created_at TEXT NOT NULL
        )",
        "CREATE INDEX IF NOT EXISTS idx_webauthn_challenges_lookup
            ON webauthn_challenges(challenge, kind)",
        "CREATE INDEX IF NOT EXISTS idx_webauthn_challenges_token
            ON webauthn_challenges(token_hash)",
    ] {
        db.prepare(sql)
            .run()
            .await
            .map_err(|_| AppError::Database)?;
    }
    Ok(())
}

pub async fn cleanup_expired_challenges(db: &D1Database) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp();
    db.prepare("DELETE FROM webauthn_challenges WHERE expires_at <= ?1")
        .bind(&[JsValue::from_f64(now as f64)])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ChallengeRow {
    pub id: String,
    pub user_id: Option<String>,
    pub challenge: String,
}

pub async fn store_challenge(
    db: &D1Database,
    user_id: Option<&str>,
    kind: &str,
    challenge: &str,
    token_hash: Option<&str>,
    ttl_secs: i64,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    db.prepare(
        "INSERT INTO webauthn_challenges (id, user_id, kind, challenge, token_hash, expires_at, consumed_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
    )
    .bind(&[
        Uuid::new_v4().to_string().into(),
        user_id.map(|v| JsValue::from_str(v)).unwrap_or(JsValue::NULL),
        kind.into(),
        challenge.into(),
        token_hash
            .map(|v| JsValue::from_str(v))
            .unwrap_or(JsValue::NULL),
        JsValue::from_f64((now.timestamp() + ttl_secs) as f64),
        now.to_rfc3339().into(),
    ])?
    .run()
    .await
    .map_err(|_| AppError::Database)?;
    Ok(())
}

async fn query_challenge(
    db: &D1Database,
    filter: &str,
    binds: &[JsValue],
    kind: &str,
) -> Result<Option<ChallengeRow>, AppError> {
    let sql = format!(
        "SELECT id, user_id, challenge FROM webauthn_challenges
         WHERE kind = ?1 AND consumed_at IS NULL AND expires_at > ?2 AND {filter}
         ORDER BY created_at DESC LIMIT 1"
    );
    let mut all = vec![
        JsValue::from_str(kind),
        JsValue::from_f64(chrono::Utc::now().timestamp() as f64),
    ];
    all.extend_from_slice(binds);
    db.prepare(sql)
        .bind(&all)?
        .first(None)
        .await
        .map_err(|_| AppError::Database)
}

pub async fn load_challenge_by_token(
    db: &D1Database,
    token_hash: &str,
    kind: &str,
) -> Result<Option<ChallengeRow>, AppError> {
    query_challenge(
        db,
        "token_hash = ?3",
        &[JsValue::from_str(token_hash)],
        kind,
    )
    .await
}

pub async fn load_challenge_for_user(
    db: &D1Database,
    user_id: &str,
    kind: &str,
    challenge: &str,
) -> Result<Option<ChallengeRow>, AppError> {
    query_challenge(
        db,
        "user_id = ?3 AND challenge = ?4",
        &[JsValue::from_str(user_id), JsValue::from_str(challenge)],
        kind,
    )
    .await
}

pub async fn consume_challenge(db: &D1Database, id: &str) -> Result<(), AppError> {
    db.prepare("UPDATE webauthn_challenges SET consumed_at = ?2 WHERE id = ?1")
        .bind(&[
            JsValue::from_str(id),
            JsValue::from_f64(chrono::Utc::now().timestamp() as f64),
        ])?
        .run()
        .await
        .map_err(|_| AppError::Database)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{signature::Signer, SigningKey};

    fn cbor_int(value: i64) -> CborValue {
        CborValue::Integer(value.into())
    }

    fn test_cose_key(key: &VerifyingKey) -> Vec<u8> {
        let point = key.to_encoded_point(false);
        let cose = CborValue::Map(vec![
            (cbor_int(1), cbor_int(2)),
            (cbor_int(3), cbor_int(-7)),
            (cbor_int(-1), cbor_int(1)),
            (cbor_int(-2), CborValue::Bytes(point.x().unwrap().to_vec())),
            (cbor_int(-3), CborValue::Bytes(point.y().unwrap().to_vec())),
        ]);
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&cose, &mut bytes).unwrap();
        bytes
    }

    #[test]
    fn base64url_roundtrip() {
        let bytes = vec![0u8, 1, 2, 250, 251, 252];
        let encoded = b64url_encode(&bytes);
        assert_eq!(b64_decode(&encoded).unwrap(), bytes);
    }

    #[test]
    fn origin_normalization() {
        assert_eq!(
            normalize_origin("https://Example.COM:443").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            normalize_origin("http://localhost:8787").unwrap(),
            "http://localhost:8787"
        );
    }

    #[test]
    fn es256_assertion_signature() {
        let signing = SigningKey::from_slice(&[7u8; 32]).unwrap();
        let verifying = signing.verifying_key();
        let public_key = test_cose_key(verifying);
        let client_data = br#"{"type":"webauthn.get","challenge":"abc"}"#;
        let auth_data = b"authenticator-data";
        let mut message = auth_data.to_vec();
        message.extend_from_slice(&Sha256::digest(client_data));
        let signature: Signature = signing.sign(&message);
        verify_assertion_signature(
            &public_key,
            auth_data,
            client_data,
            signature.to_der().as_bytes(),
        )
        .unwrap();
    }
}
