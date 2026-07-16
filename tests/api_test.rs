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
async fn recent_logs_empty_for_new_process() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "app", "cmd": "ping", "args": [],
        "restart": { "policy": "never", "backoff_secs": 2, "backoff_max_secs": 60, "max_retries": 0 }
    });
    client.post(format!("{base}/api/processes")).bearer_auth(&token).json(&body).send().await.unwrap();
    let v: serde_json::Value = client.get(format!("{base}/api/processes/app/logs?lines=10"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(v["name"], "app");
    assert!(v["lines"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn stream_logs_rejects_without_query_token() {
    // Missing required `token` query param is rejected by axum's Query
    // extractor with 400 before the handler's own 401 check ever runs.
    let (base, _, _tmp) = spawn_server().await;
    let resp = reqwest::get(format!("{base}/api/logs/stream/app")).await.unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn stream_logs_rejects_wrong_query_token() {
    let (base, _, _tmp) = spawn_server().await;
    let resp = reqwest::get(format!("{base}/api/logs/stream/app?token=wrong")).await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn stream_logs_accepts_correct_query_token() {
    let (base, token, _tmp) = spawn_server().await;
    let resp = reqwest::get(format!("{base}/api/logs/stream/app?token={token}")).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}

#[tokio::test]
async fn task_view_roundtrips_args_and_cwd() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "t2", "cmd": "cmd", "args": ["/c", "echo", "hi"], "cwd": "C:\\work",
        "schedule": "0 0 0 * * *", "enabled": false
    });
    let r = client.post(format!("{base}/api/tasks")).bearer_auth(&token)
        .json(&body).send().await.unwrap();
    assert_eq!(r.status(), 200);

    let list: serde_json::Value = client.get(format!("{base}/api/tasks"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    let view = &list["tasks"][0];
    assert_eq!(view["args"], serde_json::json!(["/c", "echo", "hi"]));
    assert_eq!(view["cwd"], "C:\\work");

    // Simulate a toggle: PUT back the view's own fields with enabled flipped.
    let put_body = serde_json::json!({
        "name": view["name"], "cmd": view["cmd"], "args": view["args"], "cwd": view["cwd"],
        "schedule": view["schedule"], "enabled": true
    });
    let r = client.put(format!("{base}/api/tasks/t2")).bearer_auth(&token)
        .json(&put_body).send().await.unwrap();
    assert_eq!(r.status(), 200);

    let list: serde_json::Value = client.get(format!("{base}/api/tasks"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    let view = &list["tasks"][0];
    assert_eq!(view["args"], serde_json::json!(["/c", "echo", "hi"]));
    assert_eq!(view["cwd"], "C:\\work");
    assert_eq!(view["enabled"], true);
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

#[tokio::test]
async fn create_process_with_autostart_starts() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "auto", "cmd": "cmd",
        "args": ["/c", "ping -n 30 127.0.0.1"],
        "autostart": true,
        "restart": { "policy": "never", "backoff_secs": 2, "backoff_max_secs": 60, "max_retries": 0 }
    });
    let r = client.post(format!("{base}/api/processes")).bearer_auth(&token)
        .json(&body).send().await.unwrap();
    assert_eq!(r.status(), 200);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let list: serde_json::Value = client.get(format!("{base}/api/processes"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(list["processes"][0]["status"], "running");
    // cleanup
    let _ = client.post(format!("{base}/api/processes/auto/stop")).bearer_auth(&token).send().await;
}

#[tokio::test]
async fn update_running_process_restarts_with_new_def() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "p", "cmd": "cmd",
        "args": ["/c", "ping -n 60 127.0.0.1"],
        "autostart": false,
        "restart": { "policy": "never", "backoff_secs": 2, "backoff_max_secs": 60, "max_retries": 0 }
    });
    client.post(format!("{base}/api/processes")).bearer_auth(&token).json(&body).send().await.unwrap();
    client.post(format!("{base}/api/processes/p/start")).bearer_auth(&token).send().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    let updated = serde_json::json!({
        "name": "p", "cmd": "cmd",
        "args": ["/c", "ping -n 60 127.0.0.1"],
        "cwd": "C:\\Windows",
        "autostart": false,
        "restart": { "policy": "never", "backoff_secs": 2, "backoff_max_secs": 60, "max_retries": 0 }
    });
    let r = client.put(format!("{base}/api/processes/p")).bearer_auth(&token)
        .json(&updated).send().await.unwrap();
    assert_eq!(r.status(), 200);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let detail: serde_json::Value = client.get(format!("{base}/api/processes/p"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(detail["status"], "running");
    assert_eq!(detail["cwd"], "C:\\Windows");
    let _ = client.post(format!("{base}/api/processes/p/stop")).bearer_auth(&token).send().await;
}
