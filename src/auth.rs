use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use worker::Env;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub exp: usize,  // Expiration time
    pub nbf: usize,  // Not before time
    pub sstamp: String,
    #[serde(default)]
    pub device: Option<String>,

    pub premium: bool,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub amr: Vec<String>,
}

impl FromRequestParts<Arc<Env>> for Claims {
    type Rejection = AppError;

    #[worker::send]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<Env>,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|auth_header| auth_header.to_str().ok())
            .and_then(|auth_value| {
                if auth_value.starts_with("Bearer ") {
                    Some(auth_value[7..].to_owned())
                } else {
                    None
                }
            })
            .or_else(|| {
                let raw = parts.headers.get(header::COOKIE)?.to_str().ok()?;
                for part in raw.split(';') {
                    let part = part.trim();
                    if let Some((k, v)) = part.split_once('=') {
                        if k.trim() == "bw_access_token" {
                            return Some(v.trim().to_string());
                        }
                    }
                }
                None
            })
            .ok_or_else(|| AppError::Unauthorized("Missing or invalid token".to_string()))?;

        let secret = state.secret("JWT_SECRET")?;

        // Decode and validate the token
        let decoding_key = DecodingKey::from_secret(secret.to_string().as_ref());
        let token_data = decode::<Claims>(&token, &decoding_key, &Validation::default())
            .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;

        let claims = token_data.claims;
        let db = crate::db::get_db(state)?;
        let row: Option<serde_json::Value> = db
            .prepare("SELECT security_stamp FROM users WHERE id = ?1")
            .bind(&[claims.sub.clone().into()])?
            .first(None)
            .await
            .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;
        let Some(row) = row else {
            return Err(AppError::Unauthorized("Invalid token".to_string()));
        };
        let current = row
            .get("security_stamp")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        if !constant_time_eq::constant_time_eq(claims.sstamp.as_bytes(), current.as_bytes()) {
            return Err(AppError::Unauthorized("Invalid token".to_string()));
        }
        if let Some(device) = claims.device.as_deref() {
            let exists: Option<i64> = db
                .prepare("SELECT 1 AS ok FROM devices WHERE user_id = ?1 AND device_identifier = ?2 LIMIT 1")
                .bind(&[claims.sub.clone().into(), device.into()])?
                .first(Some("ok"))
                .await
                .map_err(|_| AppError::Unauthorized("Invalid token".to_string()))?;
            if exists.is_none() {
                return Err(AppError::Unauthorized("Invalid token".to_string()));
            }
        }

        Ok(claims)
    }
}
