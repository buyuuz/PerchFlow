use perch::config::model::*;
use perch::core::scheduler::Scheduler;
use perch::core::state::AppState;
use perch::core::supervisor::Supervisor;
use perch::web::{build_router, WebContext};

async fn spawn_server() -> (String, String, tempfile::NamedTempFile) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let cfg = Config {
        server: ServerConfig { bind: "127.0.0.1:0".into(), token: "secret".into() },
        process: vec![], task: vec![],
    };
    let state = AppState::new(cfg, tmp.path().to_path_buf());
    let ctx = WebContext {
        supervisor: Supervisor::new(state.clone()),
        scheduler: Scheduler::new(state.clone()),
        state,
    };
    let router = build_router(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap(); });
    (format!("http://{addr}"), "secret".into(), tmp)
}

#[tokio::test]
async fn api_rejects_without_token() {
    let (base, _, _tmp) = spawn_server().await;
    let resp = reqwest::get(format!("{base}/api/processes")).await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn api_accepts_with_token() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let resp = client.get(format!("{base}/api/processes"))
        .bearer_auth(token).send().await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn create_and_list_process() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "app", "cmd": "ping", "args": ["127.0.0.1"],
        "autostart": false,
        "restart": { "policy": "never", "backoff_secs": 2, "backoff_max_secs": 60, "max_retries": 0 }
    });
    let r = client.post(format!("{base}/api/processes")).bearer_auth(&token)
        .json(&body).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let list: serde_json::Value = client.get(format!("{base}/api/processes"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(list["processes"][0]["name"], "app");
}

#[tokio::test]
async fn create_task_and_run_now() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "t", "cmd": "cmd", "args": ["/c", "exit 0"],
        "schedule": "0 0 0 * * *", "enabled": true
    });
    let r = client.post(format!("{base}/api/tasks")).bearer_auth(&token)
        .json(&body).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let run = client.post(format!("{base}/api/tasks/t/run")).bearer_auth(&token)
        .send().await.unwrap();
    let v: serde_json::Value = run.json().await.unwrap();
    assert_eq!(v["ok"], true);
}

#[tokio::test]
async fn create_task_rejects_bad_cron() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "bad", "cmd": "x", "schedule": "nope", "enabled": true
    });
    let r = client.post(format!("{base}/api/tasks")).bearer_auth(&token)
        .json(&body).send().await.unwrap();
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["ok"], false);
}
