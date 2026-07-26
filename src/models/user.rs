use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: Option<String>,
    pub avatar_color: Option<String>,
    pub email: String,
    #[serde(with = "bool_from_int")]
    pub email_verified: bool,
    pub master_password_hash: String,
    pub master_password_hint: Option<String>,
    pub key: String,
    pub private_key: String,
    pub public_key: String,
    pub kdf_type: i32,
    pub kdf_iterations: i32,
    pub security_stamp: String,
    pub created_at: String,
    pub updated_at: String,
}

mod bool_from_int {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = i64::deserialize(deserializer)?;
        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(serde::de::Error::custom("expected integer 0 or 1")),
        }
    }

    pub fn serialize<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if *value {
            serializer.serialize_i64(1)
        } else {
            serializer.serialize_i64(0)
        }
    }
}

// For /accounts/prelogin response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreloginResponse {
    pub kdf: i32,
    pub kdf_iterations: i32,
    pub kdf_memory: Option<i32>,
    pub kdf_parallelism: Option<i32>,
}

// For /accounts/register request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub name: Option<String>,
    pub email: String,
    pub master_password_hash: Option<String>,
    pub master_password_hint: Option<String>,
    pub user_symmetric_key: Option<String>,
    pub user_asymmetric_keys: Option<KeyData>,
    pub kdf: Option<i32>,
    pub kdf_iterations: Option<i32>,
    pub master_password_authentication: Option<MasterPasswordAuthentication>,
    pub master_password_unlock: Option<MasterPasswordUnlock>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterPasswordAuthentication {
    pub master_password_authentication_hash: String,
    pub kdf: Option<KdfConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterPasswordUnlock {
    pub master_key_wrapped_user_key: String,
    pub kdf: Option<KdfConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfConfig {
    pub kdf_type: Option<i32>,
    pub iterations: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyData {
    pub public_key: String,
    pub encrypted_private_key: String,
}
