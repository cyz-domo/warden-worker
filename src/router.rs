use axum::extract::DefaultBodyLimit;
use axum::http::header::{
    HeaderName, HeaderValue, CONTENT_SECURITY_POLICY, REFERRER_POLICY, X_CONTENT_TYPE_OPTIONS,
};
use axum::{
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use worker::Env;

use crate::handlers::{
    accounts, admin, ciphers, config, devices, folders, identity, import, sends, sync, two_factor,
    usage,
};

pub fn api_router(env: Env) -> Router {
    let app_state = Arc::new(env);

    Router::new()
        .route(
            "/demo.html",
            get(|| async { Html(include_str!("../static/demo.html")) }),
        )
        .route("/admin.html", get(admin_page))
        // Identity/Auth routes
        .route("/identity/accounts/prelogin", post(accounts::prelogin))
        .route(
            "/identity/accounts/prelogin/password",
            post(accounts::prelogin_password),
        )
        .route("/api/accounts/prelogin", post(accounts::prelogin))
        .route(
            "/api/accounts/prelogin/password",
            post(accounts::prelogin_password),
        )
        .route(
            "/identity/accounts/register/finish",
            post(accounts::register),
        )
        .route("/identity/connect/token", post(identity::token))
        .route(
            "/identity/accounts/register/send-verification-email",
            post(accounts::send_verification_email),
        )
        .route(
            "/identity/accounts/register/verification-email-clicked",
            post(accounts::verification_email_clicked),
        )
        .route("/api/accounts/profile", get(accounts::profile))
        .route("/api/accounts/revision-date", get(accounts::revision_date))
        .route("/api/devices", get(devices::get_devices))
        .route(
            "/api/devices/identifier/{id}",
            get(devices::get_device_by_identifier),
        )
        .route("/api/devices/knowndevice", get(devices::knowndevice))
        .route(
            "/api/devices/identifier/{id}/token",
            put(devices::device_token).post(devices::device_token),
        )
        .route(
            "/api/accounts/password",
            put(accounts::change_master_password),
        )
        .route("/api/accounts/email", put(accounts::change_email))
        .route("/api/admin/summary", get(admin::summary))
        .route("/api/admin/users", get(admin::users))
        .route(
            "/api/admin/allowlist",
            get(admin::allowlist).post(admin::add_allowlist),
        )
        .route(
            "/api/admin/allowlist/{email}",
            axum::routing::patch(admin::patch_allowlist).delete(admin::delete_allowlist),
        )
        .route("/api/two-factor", get(two_factor::two_factor_status))
        .route(
            "/api/two-factor/get-authenticator",
            post(two_factor::get_authenticator),
        )
        .route(
            "/api/two-factor/authenticator",
            post(two_factor::activate_authenticator)
                .put(two_factor::activate_authenticator_put)
                .delete(two_factor::disable_authenticator_vw),
        )
        .route(
            "/api/two-factor/authenticator/request",
            post(two_factor::authenticator_request),
        )
        .route(
            "/api/two-factor/authenticator/enable",
            post(two_factor::authenticator_enable),
        )
        .route(
            "/api/two-factor/authenticator/disable",
            post(two_factor::authenticator_disable),
        )
        .route("/api/sends", get(sends::get_sends).post(sends::post_send))
        .route("/api/sends/file/v2", post(sends::post_send_file_v2))
        .route("/api/sends/access/{access_id}", post(sends::post_access))
        .route("/api/sends/access", post(sends::post_access_with_token))
        .route(
            "/api/sends/{send_id}",
            get(sends::get_send).delete(sends::delete_send),
        )
        .route(
            "/api/sends/{send_id}/access/file/{file_id}",
            post(sends::post_access_file),
        )
        .route("/api/sends/{send_id}/{file_id}", get(sends::download_send))
        .route(
            "/api/sends/{send_id}/file/{file_id}",
            post(sends::post_send_file_v2_data).layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        .route(
            "/sends/{send_id}/file/{file_id}",
            post(sends::post_send_file_v2_data).layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // Main data sync route
        .route("/api/sync", get(sync::get_sync_data))
        // Ciphers CRUD
        .route("/api/ciphers/create", post(ciphers::create_cipher))
        .route(
            "/api/ciphers",
            post(ciphers::post_ciphers).delete(ciphers::hard_delete_ciphers_delete),
        )
        .route("/api/ciphers/import", post(import::import_data))
        .route(
            "/api/ciphers/{id}",
            put(ciphers::update_cipher).delete(ciphers::hard_delete_cipher),
        )
        .route(
            "/api/ciphers/{id}/delete",
            put(ciphers::soft_delete_cipher).post(ciphers::hard_delete_cipher_post),
        )
        .route("/api/ciphers/{id}/restore", put(ciphers::restore_cipher))
        .route(
            "/api/ciphers/delete",
            put(ciphers::soft_delete_ciphers).post(ciphers::hard_delete_ciphers),
        )
        .route("/api/ciphers/restore", put(ciphers::restore_ciphers))
        // Folders CRUD
        .route("/api/folders", post(folders::create_folder))
        .route("/api/folders/{id}", put(folders::update_folder))
        .route("/api/folders/{id}", delete(folders::delete_folder))
        .route("/api/config", get(config::config))
        .route("/api/alive", get(config::alive))
        .route("/api/now", get(config::now))
        .route("/api/version", get(config::version))
        .route("/api/webauthn", get(config::webauthn))
        .route("/api/d1/usage", get(usage::d1_usage))
        .with_state(app_state)
}

async fn admin_page() -> Response {
    let mut response = Html(include_str!("../static/admin.html")).into_response();
    let headers = response.headers_mut();
    headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    response
}
