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

    let http_req: HttpRequest = req.try_into()?;
    Ok(worker::response_to_wasm(app.call(http_req).await?)?)
}

fn should_offload(req: &Request) -> bool {
    if req.method() != worker::Method::Post {
        return false;
    }

    matches!(
        req.path().as_str(),
        "/identity/accounts/prelogin"
            | "/api/accounts/prelogin"
            | "/identity/accounts/register/finish"
            | "/identity/connect/token"
    )
}
