use crate::web::WebContext;
use axum::Router;
pub fn routes() -> Router<WebContext> { Router::new() }
