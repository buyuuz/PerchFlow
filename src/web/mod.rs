pub mod api_lang;
pub mod api_logs;
pub mod api_process;
pub mod api_project;
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
    // route_layer only applies to routes merged in before it's called, so the
    // SSE stream route (merged after) stays outside the header-auth group.
    // EventSource can't send an Authorization header, so it validates its
    // own `?token=` query param instead (see api_logs::stream_logs).
    let api = Router::new()
        .merge(api_process::routes())
        .merge(api_task::routes())
        .merge(api_project::routes())
        .merge(api_logs::routes())
        .merge(api_lang::routes())
        .route_layer(middleware::from_fn_with_state(ctx.clone(), auth::require_token))
        .merge(api_logs::stream_routes())
        .merge(api_lang::public_routes());

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
