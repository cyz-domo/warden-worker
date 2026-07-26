use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportCipher {
    #[serde(rename = "type")]
    pub r#type: i32,
    pub folder_id: Option<String>,
    pub organization_id: Option<String>,
    pub name: String,
    pub key: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    pub login: Option<Value>,
    pub card: Option<Value>,
    pub identity: Option<Value>,
    pub ssh_key: Option<Value>,
    pub secure_note: Option<Value>,
    pub fields: Option<Value>,
    pub password_history: Option<Value>,
    pub reprompt: Option<i32>,
    pub last_known_revision_date: Option<String>,
    pub encrypted_for: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportFolder {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FolderRelationship {
    pub key: usize,
    pub value: usize,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    #[serde(default)]
    pub ciphers: Vec<ImportCipher>,
    #[serde(default)]
    pub folders: Vec<ImportFolder>,
    #[serde(default)]
    pub folder_relationships: Vec<FolderRelationship>,
}

#[cfg(test)]
mod tests {
    use super::ImportRequest;

    #[test]
    fn accepts_import_payload_with_generated_folder_ids() {
        let payload: ImportRequest = serde_json::from_str(
            r#"{
                "ciphers": [{"type": 1, "name": "item", "login": null}],
                "folders": [{"name": "folder"}],
                "folderRelationships": [{"key": 0, "value": 0}]
            }"#,
        )
        .expect("import payload should deserialize");

        assert!(payload.ciphers[0].encrypted_for.is_none());
        assert!(!payload.ciphers[0].favorite);
        assert!(payload.folders[0].id.is_none());
    }
}
