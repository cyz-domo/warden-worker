use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use serde_json::{json, Value};
use std::sync::Arc;
use worker::Env;

use crate::{
    auth::Claims,
    db,
    error::AppError,
    models::{
        folder::{Folder, FolderResponse},
        send::{send_to_json, SendDBModel},
        sync::Profile,
        user::User,
    },
    two_factor,
};

pub struct RawJson(pub String);

impl IntoResponse for RawJson {
    fn into_response(self) -> Response {
        ([(header::CONTENT_TYPE, "application/json")], self.0).into_response()
    }
}

#[worker::send]
pub async fn get_sync_data(
    claims: Claims,
    State(env): State<Arc<Env>>,
) -> Result<RawJson, AppError> {
    let user_id = claims.sub;
    let db = db::get_db(&env)?;

    // Fetch profile
    let user: User = db
        .prepare("SELECT * FROM users WHERE id = ?1")
        .bind(&[user_id.clone().into()])?
        .first(None)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Fetch folders
    let folders_db: Vec<Folder> = db
        .prepare("SELECT * FROM folders WHERE user_id = ?1")
        .bind(&[user_id.clone().into()])?
        .all()
        .await?
        .results()?;

    let folders: Vec<FolderResponse> = folders_db.into_iter().map(|f| f.into()).collect();

    // Fetch ciphers
    let cipher_json: String = db
        .prepare(
            "SELECT COALESCE(json_group_array(json(sub.cipher_json)), '[]') AS ciphers_json
             FROM (
               SELECT json_object(
                 'object', 'cipher', 'id', c.id, 'userId', c.user_id,
                 'organizationId', c.organization_id, 'folderId', c.folder_id,
                 'type', c.type, 'favorite', json(CASE WHEN c.favorite THEN 'true' ELSE 'false' END),
                 'edit', json('true'), 'viewPassword', json('true'),
                 'permissions', json_object('delete', json('true'), 'restore', json('true')),
                 'organizationUseTotp', json('false'), 'collectionIds', json('[]'),
                 'revisionDate', c.updated_at, 'creationDate', c.created_at,
                 'deletedDate', c.deleted_at, 'archivedDate', c.archived_at,
                 'name', json_extract(c.data, '$.name'),
                 'notes', json_extract(c.data, '$.notes'),
                 'fields', COALESCE(json_extract(c.data, '$.fields'), json('[]')),
                 'passwordHistory', COALESCE(json_extract(c.data, '$.passwordHistory'), json('[]')),
                 'reprompt', COALESCE(json_extract(c.data, '$.reprompt'), 0),
                 'login', CASE WHEN c.type = 1 THEN json_extract(c.data, '$.login') ELSE NULL END,
                 'secureNote', CASE WHEN c.type = 2 THEN json_extract(c.data, '$.secureNote') ELSE NULL END,
                 'card', CASE WHEN c.type = 3 THEN json_extract(c.data, '$.card') ELSE NULL END,
                 'identity', CASE WHEN c.type = 4 THEN json_extract(c.data, '$.identity') ELSE NULL END,
                 'sshKey', CASE WHEN c.type = 5 THEN json_extract(c.data, '$.sshKey') ELSE NULL END,
                 'key', json_extract(c.data, '$.key')
               ) AS cipher_json
               FROM ciphers c WHERE c.user_id = ?1 ORDER BY c.updated_at DESC
             ) sub",
        )
        .bind(&[user_id.clone().into()])?
        .first(Some("ciphers_json"))
        .await
        .map_err(|_| AppError::Database)?
        .unwrap_or_else(|| "[]".to_string());

    let send_rows: Vec<Value> = db
        .prepare("SELECT * FROM sends WHERE user_id = ?1 ORDER BY updated_at DESC")
        .bind(&[user_id.clone().into()])?
        .all()
        .await?
        .results()?;
    let sends = send_rows
        .into_iter()
        .filter_map(|v| match serde_json::from_value::<SendDBModel>(v.clone()) {
            Ok(send) => Some(send),
            Err(error) => {
                let send_id = v.get("id").and_then(Value::as_str).unwrap_or("unknown");
                log::warn!("Cannot parse send {send_id}: {error}");
                None
            }
        })
        .map(|s| send_to_json(&s))
        .collect::<Vec<_>>();

    let user_decryption = json!({
        "masterPasswordUnlock": {
            "kdf": {
                "kdfType": user.kdf_type,
                "iterations": user.kdf_iterations,
                "memory": null,
                "parallelism": null
            },
            "masterKeyEncryptedUserKey": user.key,
            "masterKeyWrappedUserKey": user.key,
            "salt": user.email
        }
    });

    let time = chrono::DateTime::parse_from_rfc3339(&user.created_at)
        .map_err(|_| AppError::Internal)?
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let profile = Profile {
        id: user.id,
        name: user.name,
        avatar_color: user.avatar_color,
        email: user.email,
        master_password_hint: user.master_password_hint,
        security_stamp: user.security_stamp,
        object: "profile".to_string(),
        premium: true,
        premium_from_organization: false,
        email_verified: true,
        force_password_reset: false,
        two_factor_enabled: two_factor::is_authenticator_enabled(&db, &user_id).await?,
        uses_key_connector: false,
        creation_date: time,
        key: user.key,
        private_key: user.private_key,
    };

    let profile = serde_json::to_string(&profile).map_err(|_| AppError::Internal)?;
    let folders = serde_json::to_string(&folders).map_err(|_| AppError::Internal)?;
    let sends = serde_json::to_string(&sends).map_err(|_| AppError::Internal)?;
    let user_decryption =
        serde_json::to_string(&user_decryption).map_err(|_| AppError::Internal)?;
    Ok(RawJson(format!(
        "{{\"profile\":{profile},\"folders\":{folders},\"collections\":[],\"policies\":[],\"ciphers\":{cipher_json},\"sends\":{sends},\"domains\":null,\"userDecryption\":{user_decryption},\"object\":\"sync\"}}"
    )))
}
