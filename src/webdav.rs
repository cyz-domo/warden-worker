use base64::{engine::general_purpose, Engine as _};
use js_sys::{ArrayBuffer, Object, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::error::AppError;

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = fetch)]
    fn web_fetch(url: &str, init: &Object) -> js_sys::Promise;
}

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:"><d:prop><d:displayname/><d:getcontentlength/><d:resourcetype/></d:prop></d:propfind>"#;
const MAX_REMOTE_FILE_BYTES: usize = 64 * 1024 * 1024;

pub struct WebDavClient {
    base: String,
    authorization: Option<String>,
}

fn set(object: &Object, key: &str, value: &wasm_bindgen::JsValue) {
    let _ = Reflect::set(object, &wasm_bindgen::JsValue::from_str(key), value);
}

fn validate_base(url: &str) -> Result<String, AppError> {
    let trimmed = url.trim();
    let invalid = |message: &str| AppError::BadRequest(message.to_string());
    if !trimmed.starts_with("https://") {
        return Err(invalid(
            "The WebDAV address must be a full https:// directory URL.",
        ));
    }
    if trimmed
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(invalid("The WebDAV address contains whitespace."));
    }
    if trimmed.contains('?') || trimmed.contains('#') {
        return Err(invalid(
            "The WebDAV address must not contain a query string or fragment.",
        ));
    }
    let rest = &trimmed[8..];
    let authority = rest.split('/').next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err(invalid(
            "The WebDAV address needs a host without credentials.",
        ));
    }
    let host = authority
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    if host.is_empty()
        || host == "localhost"
        || host == "0.0.0.0"
        || host == "::1"
        || host.contains(':')
        || host.starts_with("127.")
        || host.starts_with("169.254.")
        || host.ends_with(".local")
        || host.ends_with(".localhost")
        || host.ends_with(".internal")
    {
        return Err(invalid(
            "The WebDAV host is not reachable from Workers or is not allowed.",
        ));
    }
    Ok(if trimmed.ends_with('/') {
        trimmed.to_string()
    } else {
        format!("{trimmed}/")
    })
}

impl WebDavClient {
    pub fn new(
        url: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, AppError> {
        let base = validate_base(url)?;
        let authorization = match (username, password) {
            (Some(username), Some(password)) if !username.is_empty() => Some(format!(
                "Basic {}",
                general_purpose::STANDARD.encode(format!("{username}:{password}"))
            )),
            _ => None,
        };
        Ok(Self {
            base,
            authorization,
        })
    }

    pub fn base_url(&self) -> String {
        self.base.clone()
    }

    fn file_url(&self, name: &str) -> String {
        format!("{}{}", self.base, name)
    }

    async fn request(
        &self,
        method: &str,
        url: &str,
        content_type: Option<&str>,
        depth: Option<&str>,
        body: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>), AppError> {
        let init = Object::new();
        set(&init, "method", &wasm_bindgen::JsValue::from_str(method));
        set(
            &init,
            "redirect",
            &wasm_bindgen::JsValue::from_str("follow"),
        );
        let headers = Object::new();
        if let Some(authorization) = &self.authorization {
            set(
                &headers,
                "authorization",
                &wasm_bindgen::JsValue::from_str(authorization),
            );
        }
        if let Some(content_type) = content_type {
            set(
                &headers,
                "content-type",
                &wasm_bindgen::JsValue::from_str(content_type),
            );
        }
        if let Some(depth) = depth {
            set(&headers, "depth", &wasm_bindgen::JsValue::from_str(depth));
        }
        set(&init, "headers", headers.as_ref());
        if let Some(body) = body {
            let array = Uint8Array::new_from_slice(body);
            set(&init, "body", array.as_ref());
        }

        let promise = web_fetch(url, &init);
        let response_value = JsFuture::from(promise)
            .await
            .map_err(|error| AppError::BadRequest(format!("WebDAV request failed: {error:?}")))?;
        let response: web_sys::Response = response_value.dyn_into().map_err(|_| {
            AppError::BadRequest("WebDAV returned an invalid response.".to_string())
        })?;
        let status = response.status();
        let buffer = JsFuture::from(response.array_buffer().map_err(|_| AppError::Internal)?)
            .await
            .map_err(|_| AppError::BadRequest("WebDAV response could not be read.".to_string()))?;
        let buffer: ArrayBuffer = buffer.dyn_into().map_err(|_| AppError::Internal)?;
        let bytes = Uint8Array::new(&buffer).to_vec();
        Ok((status, bytes))
    }

    fn status_error(action: &str, status: u16, body: &[u8]) -> AppError {
        let detail = String::from_utf8_lossy(body);
        let detail = detail.trim();
        let detail = if detail.is_empty() {
            String::new()
        } else {
            format!(": {}", detail.chars().take(200).collect::<String>())
        };
        AppError::BadRequest(format!("WebDAV {action} failed with HTTP {status}{detail}"))
    }

    async fn mkcol(&self, url: &str) -> Result<(), AppError> {
        let (status, body) = self.request("MKCOL", url, None, None, None).await?;
        match status {
            200 | 201 | 204 | 301 | 403 | 405 => Ok(()),
            _ => Err(Self::status_error("MKCOL", status, &body)),
        }
    }

    pub async fn ensure_dir(&self) -> Result<(), AppError> {
        let rest = &self.base[8..];
        let Some(slash) = rest.find('/') else {
            return self.mkcol(&self.base).await;
        };
        let authority = &rest[..slash];
        let path = &rest[slash + 1..];
        let mut prefix = format!("https://{authority}");
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            prefix.push('/');
            prefix.push_str(segment);
            prefix.push('/');
            self.mkcol(&prefix).await?;
        }
        Ok(())
    }

    pub async fn put(&self, name: &str, body: &[u8]) -> Result<(), AppError> {
        let url = self.file_url(name);
        let mut result = self
            .request(
                "PUT",
                &url,
                Some("application/octet-stream"),
                None,
                Some(body),
            )
            .await?;
        if result.0 == 409 {
            self.ensure_dir().await?;
            result = self
                .request(
                    "PUT",
                    &url,
                    Some("application/octet-stream"),
                    None,
                    Some(body),
                )
                .await?;
        }
        match result.0 {
            200 | 201 | 204 => Ok(()),
            status => Err(Self::status_error("upload", status, &result.1)),
        }
    }

    pub async fn get(&self, name: &str) -> Result<Vec<u8>, AppError> {
        let (status, body) = self
            .request("GET", &self.file_url(name), None, None, None)
            .await?;
        match status {
            200 => {
                if body.len() > MAX_REMOTE_FILE_BYTES {
                    return Err(AppError::BadRequest(
                        "The remote backup is larger than 64 MiB.".to_string(),
                    ));
                }
                Ok(body)
            }
            404 => Err(AppError::NotFound("Backup file not found.".to_string())),
            status => Err(Self::status_error("download", status, &body)),
        }
    }

    pub async fn delete(&self, name: &str) -> Result<(), AppError> {
        let (status, body) = self
            .request("DELETE", &self.file_url(name), None, None, None)
            .await?;
        match status {
            200 | 204 | 404 => Ok(()),
            status => Err(Self::status_error("delete", status, &body)),
        }
    }

    /// Returns remote file names in the configured collection, newest name first.
    pub async fn list(&self) -> Result<Vec<String>, AppError> {
        let (status, body) = self
            .request(
                "PROPFIND",
                &self.base,
                Some("application/xml; charset=utf-8"),
                Some("1"),
                Some(PROPFIND_BODY.as_bytes()),
            )
            .await?;
        match status {
            200 | 207 => {}
            404 => return Ok(Vec::new()),
            status => return Err(Self::status_error("list", status, &body)),
        }
        let text = String::from_utf8_lossy(&body);
        let mut names = Vec::new();
        let mut cursor = 0usize;
        while let Some(start) = text[cursor..].find('<') {
            let start = cursor + start;
            let Some(end) = text[start..].find('>') else {
                break;
            };
            let end = start + end;
            let raw = text[start + 1..end].trim();
            let tag = raw
                .trim_end_matches('/')
                .rsplit(':')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            cursor = end + 1;
            if raw.starts_with('/') || tag != "href" {
                continue;
            }
            let Some(close) = text[cursor..].find('<') else {
                break;
            };
            let value = text[cursor..cursor + close].trim();
            cursor += close;
            let name = value
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default();
            if name.starts_with("warden-") && name.ends_with(".enc") {
                names.push(name.to_string());
            }
        }
        names.sort();
        names.dedup();
        names.reverse();
        Ok(names)
    }

    pub async fn test(&self) -> Result<(), AppError> {
        self.ensure_dir().await?;
        self.list().await?;
        Ok(())
    }
}
