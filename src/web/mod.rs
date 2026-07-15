pub mod api_logs;
pub mod api_process;
pub mod api_task;
pub mod assets;
pub mod auth;

use crate::core::scheduler::Scheduler;
use crate::core::state::SharedState;
use crate::core::supervisor::Supervisor;
use crate::error::{PerchError, Result};
use axum::{middleware, Router};
use std::sync::Arc;

#[derive(Clone)]
pub struct WebContext {
    pub state: SharedState,
    pub supervisor: Arc<Supervisor>,
    pub scheduler: Arc<Scheduler>,
}

pub fn build_router(ctx: WebContext) -> Router {
    let api = Router::new()
        .merge(api_process::routes())
        .merge(api_task::routes())
        .merge(api_logs::routes())
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth::require_token));

    Router::new()
        .nest("/api", api)
        .merge(assets::routes())
        .with_state(ctx)
}

pub async fn serve(ctx: WebContext, bind: &str) -> Result<()> {
    let router = build_router(ctx);
    let listener = tokio::net::TcpListener::bind(bind).await
        .map_err(|e| PerchError::Web(format!("bind {bind}: {e}")))?;
    axum::serve(listener, router).await
        .map_err(|e| PerchError::Web(format!("serve: {e}")))?;
    Ok(())
}
