use axum::extract::DefaultBodyLimit;
use tower_http::cors::{Any, CorsLayer};
use tower_service::Service;
use worker::{durable_object, DurableObject, Env, HttpRequest, Request, Response, Result, State};

use crate::router;

#[durable_object]
pub struct HeavyDo {
    state: State,
    env: Env,
}

impl DurableObject for HeavyDo {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Debug);
        let _ = &self.state;

        let http_req: HttpRequest = req.try_into()?;
        let cors = CorsLayer::new()
            .allow_methods(Any)
            .allow_headers(Any)
            .allow_origin(Any);

        const BODY_LIMIT: usize = 5 * 1024 * 1024;
        let mut app = router::api_router(self.env.clone())
            .layer(cors)
            .layer(DefaultBodyLimit::max(BODY_LIMIT));

        app.call(http_req).await?.try_into()
    }
}
