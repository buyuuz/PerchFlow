use perch::config::model::*;
use perch::core::scheduler::Scheduler;
use perch::core::state::AppState;
use perch::core::supervisor::Supervisor;
use perch::web::{build_router, WebContext};
use std::path::PathBuf;

async fn spawn_server() -> (String, String) {
    let cfg = Config {
        server: ServerConfig { bind: "127.0.0.1:0".into(), token: "secret".into() },
        process: vec![], task: vec![],
    };
    let state = AppState::new(cfg, PathBuf::from("perch.toml"));
    let ctx = WebContext {
        supervisor: Supervisor::new(state.clone()),
        scheduler: Scheduler::new(state.clone()),
        state,
    };
    let router = build_router(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap(); });
    (format!("http://{addr}"), "secret".into())
}

#[tokio::test]
async fn api_rejects_without_token() {
    let (base, _) = spawn_server().await;
    let resp = reqwest::get(format!("{base}/api/ping")).await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn api_accepts_with_token() {
    let (base, token) = spawn_server().await;
    let client = reqwest::Client::new();
    let resp = client.get(format!("{base}/api/ping"))
        .bearer_auth(token).send().await.unwrap();
    assert_eq!(resp.status(), 200);
}
