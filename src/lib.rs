use tower_http::cors::{Any, CorsLayer};
use tower_service::Service;
use worker::*;

mod auth;
mod crypto;
mod db;
mod durable;
mod error;
mod handlers;
mod models;
mod ratelimit;
mod router;
mod two_factor;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<web_sys::Response> {
    // Set up logging
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Debug);

    // Allow all origins for CORS, which is typical for a public API like Bitwarden's.
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    if should_offload(&req) {
        if let Ok(namespace) = env.durable_object("HEAVY_DO") {
            if let Ok(stub) = namespace.get_by_name("global") {
                let response = stub.fetch_with_request(req).await?;
                return Ok(response.into());
            }
        }
    }

    let mut app = router::api_router(env).layer(cors);

    let mut http_req: HttpRequest = req.try_into()?;
    rewrite_admin_prefix(&mut http_req);
    Ok(worker::response_to_wasm(app.call(http_req).await?)?)
}

fn should_offload(req: &Request) -> bool {
    if req.method() != worker::Method::Post {
        return false;
    }

    let path = req.path();
    matches!(
        strip_admin_prefix(&path),
        "/identity/accounts/prelogin"
            | "/api/accounts/prelogin"
            | "/identity/accounts/register/finish"
            | "/identity/connect/token"
    )
}

const ADMIN_PREFIX: &str = "/admin";

/// Upstream serves its admin console from `/admin`, so a client once pointed at
/// that path can persist it as its environment base URL and keep prefixing every
/// API call with `/admin`. Accept those calls instead of failing on them.
fn strip_admin_prefix(path: &str) -> &str {
    match path.strip_prefix(ADMIN_PREFIX) {
        Some(rest)
            if rest.starts_with("/api/")
                || rest.starts_with("/identity/")
                || rest.starts_with("/sends/") =>
        {
            rest
        }
        _ => path,
    }
}

pub(crate) fn rewrite_admin_prefix(req: &mut HttpRequest) {
    let path = req.uri().path();
    let stripped = strip_admin_prefix(path);
    if stripped == path {
        return;
    }

    let path_and_query = match req.uri().query() {
        Some(query) => format!("{stripped}?{query}"),
        None => stripped.to_string(),
    };
    let mut builder = http::Uri::builder().path_and_query(path_and_query);
    if let (Some(scheme), Some(authority)) = (req.uri().scheme(), req.uri().authority()) {
        builder = builder
            .scheme(scheme.as_str())
            .authority(authority.as_str());
    }
    if let Ok(uri) = builder.build() {
        *req.uri_mut() = uri;
    }
}
