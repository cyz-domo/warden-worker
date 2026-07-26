use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Value};

// This struct represents the data stored in the `data` column of the `ciphers` table.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CipherData {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_key: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_note: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_history: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reprompt: Option<i32>,
}

// Custom deserialization function for booleans
fn deserialize_bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    // A visitor is used to handle different data types
    struct BoolOrIntVisitor;

    impl<'de> de::Visitor<'de> for BoolOrIntVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or an integer 0 or 1")
        }

        // Handles boolean values
        fn visit_bool<E>(self, value: bool) -> Result<bool, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        // Handles integer values (0 or 1)
        fn visit_u64<E>(self, value: u64) -> Result<bool, E>
        where
            E: de::Error,
        {
            match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(de::Error::invalid_value(
                    de::Unexpected::Unsigned(value),
                    &"0 or 1",
                )),
            }
        }
    }

    deserializer.deserialize_any(BoolOrIntVisitor)
}

// The struct that is stored in the database and used in handlers.
// For serialization to JSON for the client, we implement a custom `Serialize`.
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cipher {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(rename = "type")]
    pub r#type: i32,
    pub data: Value,
    pub key: Option<String>,
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,

    // Bitwarden specific field for API responses
    #[serde(default = "default_object")]
    pub object: String,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub organization_use_totp: bool,
    #[serde(default = "default_true")]
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub edit: bool,
    #[serde(default = "default_true")]
    #[serde(deserialize_with = "deserialize_bool_from_int")]
    pub view_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CipherDBModel {
    pub id: String,
    pub user_id: String,
    pub organization_id: Option<String>,
    pub r#type: i32,
    #[serde(deserialize_with = "deserialize_json_text")]
    pub data: String,
    pub key: Option<String>,
    #[serde(deserialize_with = "deserialize_i32_from_bool_or_number")]
    pub favorite: i32,
    pub folder_id: Option<String>,
    pub deleted_at: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn deserialize_json_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(text) => Ok(text),
        other if other.is_object() || other.is_array() => serde_json::to_string(&other)
            .map_err(|error| de::Error::custom(format!("invalid cipher data: {error}"))),
        _other => Err(de::Error::custom(
            "cipher data must be a JSON string or object",
        )),
    }
}

fn deserialize_i32_from_bool_or_number<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Bool(value) => Ok(i32::from(value)),
        Value::Number(value) => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| de::Error::custom("cipher favorite must be an integer")),
        Value::String(value) => value
            .parse::<i32>()
            .map_err(|_| de::Error::custom("cipher favorite must be an integer")),
        _ => Err(de::Error::custom(
            "cipher favorite must be a boolean or integer",
        )),
    }
}

impl Into<Cipher> for CipherDBModel {
    fn into(self) -> Cipher {
        Cipher {
            id: self.id,
            user_id: Some(self.user_id),
            organization_id: self.organization_id,
            r#type: self.r#type,
            data: serde_json::from_str(&self.data).unwrap_or_default(),
            key: self.key,
            favorite: match self.favorite {
                0 => false,
                _ => true,
            },
            folder_id: self.folder_id,
            deleted_at: self.deleted_at,
            archived_at: self.archived_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            object: default_object(),
            organization_use_totp: false,
            edit: true,
            view_password: true,
            collection_ids: None,
        }
    }
}

impl Serialize for Cipher {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut response_map = Map::new();

        response_map.insert("object".to_string(), json!("cipherDetails"));
        response_map.insert("id".to_string(), json!(self.id));
        if self.user_id.is_some() {
            response_map.insert("userId".to_string(), json!(self.user_id));
        }
        response_map.insert("organizationId".to_string(), json!(self.organization_id));
        response_map.insert("folderId".to_string(), json!(self.folder_id));
        response_map.insert("type".to_string(), json!(self.r#type));
        response_map.insert("key".to_string(), json!(self.key));
        response_map.insert("favorite".to_string(), json!(self.favorite));
        response_map.insert("edit".to_string(), json!(self.edit));
        response_map.insert(
            "permissions".to_string(),
            json!({ "delete": self.edit, "restore": self.edit }),
        );
        response_map.insert("viewPassword".to_string(), json!(self.view_password));
        response_map.insert(
            "organizationUseTotp".to_string(),
            json!(self.organization_use_totp),
        );
        response_map.insert("collectionIds".to_string(), json!(self.collection_ids));
        response_map.insert("revisionDate".to_string(), json!(self.updated_at));
        response_map.insert("creationDate".to_string(), json!(self.created_at));
        response_map.insert("deletedDate".to_string(), json!(self.deleted_at));
        response_map.insert("archivedDate".to_string(), json!(self.archived_at));

        if let Some(data_obj) = self.data.as_object() {
            let data_clone = data_obj.clone();

            response_map.insert(
                "name".to_string(),
                data_clone.get("name").cloned().unwrap_or(Value::Null),
            );
            response_map.insert(
                "notes".to_string(),
                data_clone.get("notes").cloned().unwrap_or(Value::Null),
            );
            response_map.insert("fields".to_string(), clean_fields(data_clone.get("fields")));
            response_map.insert(
                "passwordHistory".to_string(),
                clean_password_history(data_clone.get("passwordHistory")),
            );
            response_map.insert(
                "reprompt".to_string(),
                data_clone
                    .get("reprompt")
                    .cloned()
                    .unwrap_or(Value::Number(serde_json::Number::from_f64(0.0).unwrap())),
            );

            let mut login = Value::Null;
            let mut secure_note = Value::Null;
            let mut card = Value::Null;
            let mut identity = Value::Null;

            match self.r#type {
                1 => login = clean_login(data_clone.get("login")),
                2 => secure_note = clean_secure_note(data_clone.get("secureNote")),
                3 => card = data_clone.get("card").cloned().unwrap_or(Value::Null),
                4 => identity = data_clone.get("identity").cloned().unwrap_or(Value::Null),
                _ => {}
            }

            let uri = if self.r#type == 1 {
                login_uri(&login)
            } else {
                Value::Null
            };
            response_map.insert("login".to_string(), login);
            response_map.insert("secureNote".to_string(), secure_note);
            response_map.insert("card".to_string(), card);
            response_map.insert("identity".to_string(), identity);
            response_map.insert(
                "sshKey".to_string(),
                if self.r#type == 5 {
                    clean_ssh_key(data_clone.get("sshKey"))
                } else {
                    Value::Null
                },
            );
            response_map.insert("uri".to_string(), uri);
        } else {
            response_map.insert("name".to_string(), Value::Null);
            response_map.insert("notes".to_string(), Value::Null);
            response_map.insert("fields".to_string(), Value::Null);
            response_map.insert("passwordHistory".to_string(), Value::Null);
            response_map.insert("reprompt".to_string(), Value::Null);
            response_map.insert("login".to_string(), Value::Null);
            response_map.insert("secureNote".to_string(), Value::Null);
            response_map.insert("card".to_string(), Value::Null);
            response_map.insert("identity".to_string(), Value::Null);
            response_map.insert("sshKey".to_string(), Value::Null);
            response_map.insert("uri".to_string(), Value::Null);
        }

        Value::Object(response_map).serialize(serializer)
    }
}

fn clean_fields(value: Option<&Value>) -> Value {
    let Some(Value::Array(fields)) = value else {
        return Value::Array(Vec::new());
    };
    Value::Array(
        fields
            .iter()
            .map(|field| {
                let mut field = field.clone();
                if let Value::Object(object) = &mut field {
                    let field_type = object.get("type").and_then(|value| {
                        value
                            .as_i64()
                            .or_else(|| value.as_str()?.parse::<i64>().ok())
                    });
                    object.insert(
                        "type".to_string(),
                        json!(field_type
                            .filter(|value| (0..=255).contains(value))
                            .unwrap_or(1)),
                    );
                }
                field
            })
            .collect(),
    )
}

fn clean_password_history(value: Option<&Value>) -> Value {
    let Some(Value::Array(history)) = value else {
        return Value::Array(Vec::new());
    };
    Value::Array(
        history
            .iter()
            .filter_map(|entry| {
                let Value::Object(object) = entry else {
                    return None;
                };
                if !object.get("password").is_some_and(Value::is_string) {
                    return None;
                }
                let mut entry = entry.clone();
                if let Value::Object(object) = &mut entry {
                    object
                        .entry("lastUsedDate".to_string())
                        .or_insert_with(|| json!("1970-01-01T00:00:00.000000Z"));
                }
                Some(entry)
            })
            .collect(),
    )
}

fn clean_login(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    let mut login = value.clone();
    if let Value::Object(object) = &mut login {
        if let Some(Value::Array(uris)) = object.get_mut("uris") {
            for uri in uris {
                if let Value::Object(uri) = uri {
                    let normalized = uri.get("match").and_then(|value| {
                        value
                            .as_i64()
                            .or_else(|| value.as_str()?.parse::<i64>().ok())
                    });
                    uri.insert(
                        "match".to_string(),
                        normalized
                            .filter(|value| (0..=255).contains(value))
                            .map_or(Value::Null, |value| json!(value)),
                    );
                }
            }
        }
    }
    login
}

fn login_uri(login: &Value) -> Value {
    login
        .get("uris")
        .and_then(Value::as_array)
        .and_then(|uris| uris.first())
        .and_then(|uri| uri.get("uri"))
        .cloned()
        .unwrap_or(Value::Null)
}

fn clean_secure_note(value: Option<&Value>) -> Value {
    match value {
        Some(Value::Object(object)) if object.get("type").is_some_and(Value::is_number) => {
            Value::Object(object.clone())
        }
        _ => json!({ "type": 0 }),
    }
}

fn clean_ssh_key(value: Option<&Value>) -> Value {
    let Some(Value::Object(object)) = value else {
        return Value::Null;
    };
    if ["keyFingerprint", "privateKey", "publicKey"]
        .iter()
        .all(|name| {
            object
                .get(*name)
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty())
        })
    {
        Value::Object(object.clone())
    } else {
        Value::Null
    }
}

fn default_object() -> String {
    "cipher".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{Cipher, CipherDBModel};
    use serde_json::{json, Value};

    #[test]
    fn cipher_serialization_includes_permissions_delete() {
        let cipher = Cipher {
            id: "test-id".to_string(),
            user_id: Some("user-1".to_string()),
            organization_id: None,
            r#type: 1,
            data: json!({
                "name": "Example",
                "notes": null,
                "login": { "username": "u", "password": "p" }
            }),
            key: None,
            favorite: false,
            folder_id: None,
            deleted_at: None,
            archived_at: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
            object: "cipher".to_string(),
            organization_use_totp: false,
            edit: true,
            view_password: true,
            collection_ids: None,
        };

        let value = serde_json::to_value(cipher).expect("serialize cipher");

        let permissions = value
            .get("permissions")
            .and_then(Value::as_object)
            .expect("permissions object");

        assert_eq!(
            permissions.get("delete"),
            Some(&Value::Bool(true)),
            "permissions.delete must exist and be true when edit=true"
        );
        assert_eq!(
            permissions.get("restore"),
            Some(&Value::Bool(true)),
            "permissions.restore must exist and be true when edit=true"
        );
    }

    #[test]
    fn cipher_db_model_accepts_object_data_and_boolean_favorite() {
        let row = json!({
            "id": "cipher-id",
            "user_id": "user-id",
            "organization_id": null,
            "type": 1,
            "data": {"name": "Example", "login": {"username": "u"}},
            "key": null,
            "favorite": true,
            "folder_id": null,
            "deleted_at": null,
            "created_at": "2026-01-01T00:00:00.000Z",
            "updated_at": "2026-01-01T00:00:00.000Z"
        });

        let model: CipherDBModel = serde_json::from_value(row).expect("cipher row");
        let cipher: Cipher = model.into();
        assert_eq!(cipher.data["name"], "Example");
        assert!(cipher.favorite);
    }

    #[test]
    fn cipher_db_model_accepts_json_text_and_integer_favorite() {
        let row = json!({
            "id": "cipher-id",
            "user_id": "user-id",
            "organization_id": null,
            "type": 1,
            "data": "{\"name\":\"Example\",\"login\":null}",
            "key": null,
            "favorite": 0,
            "folder_id": null,
            "deleted_at": null,
            "created_at": "2026-01-01T00:00:00.000Z",
            "updated_at": "2026-01-01T00:00:00.000Z"
        });

        let model: CipherDBModel = serde_json::from_value(row).expect("cipher row");
        let cipher: Cipher = model.into();
        assert_eq!(cipher.data["name"], "Example");
        assert!(!cipher.favorite);
    }

    #[test]
    fn cipher_serialization_normalizes_mobile_compatibility_fields() {
        let cipher = Cipher {
            id: "cipher-id".to_string(),
            user_id: Some("user-id".to_string()),
            organization_id: None,
            r#type: 1,
            data: json!({
                "name": "Login",
                "login": {
                    "uris": [{"uri": "https://example.com", "match": "0"}]
                },
                "fields": [{"type": "2", "name": "field"}],
                "passwordHistory": [{"password": "old"}, {"password": null}]
            }),
            key: Some("encrypted-cipher-key".to_string()),
            favorite: false,
            folder_id: None,
            deleted_at: None,
            archived_at: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
            object: "cipher".to_string(),
            organization_use_totp: false,
            edit: true,
            view_password: true,
            collection_ids: None,
        };

        let value = serde_json::to_value(cipher).expect("serialize cipher");
        assert_eq!(value["object"], "cipherDetails");
        assert_eq!(value["key"], "encrypted-cipher-key");
        assert_eq!(value["uri"], "https://example.com");
        assert_eq!(value["login"]["uris"][0]["match"], 0);
        assert_eq!(value["fields"][0]["type"], 2);
        assert_eq!(value["passwordHistory"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn cipher_serialization_protects_invalid_mobile_cipher_data() {
        let cipher = Cipher {
            id: "cipher-id".to_string(),
            user_id: Some("user-id".to_string()),
            organization_id: None,
            r#type: 2,
            data: json!({"name": "Note", "secureNote": null}),
            key: None,
            favorite: false,
            folder_id: None,
            deleted_at: None,
            archived_at: None,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            updated_at: "2026-01-01T00:00:00.000Z".to_string(),
            object: "cipher".to_string(),
            organization_use_totp: false,
            edit: true,
            view_password: true,
            collection_ids: None,
        };

        let value = serde_json::to_value(cipher).expect("serialize cipher");
        assert_eq!(value["secureNote"]["type"], 0);
    }
}

// Represents the "Cipher" object within the incoming request payload.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherRequestData {
    #[serde(rename = "type")]
    pub r#type: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_key: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_note: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_history: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reprompt: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_known_revision_date: Option<String>,
}

// Represents the full request payload for creating a cipher.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateCipherRequest {
    pub cipher: CipherRequestData,
    #[serde(default)]
    pub collection_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherRequestFlat {
    #[serde(flatten)]
    pub cipher: CipherRequestData,
    #[serde(default)]
    pub collection_ids: Vec<String>,
}
