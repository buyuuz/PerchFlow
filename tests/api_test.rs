use perch::config::model::*;
use perch::core::scheduler::Scheduler;
use perch::core::state::AppState;
use perch::core::supervisor::Supervisor;
use perch::web::{build_router, WebContext};

async fn spawn_server() -> (String, String, tempfile::TempDir) {
    // A NamedTempFile holds the config handle open on Windows and prevents
    // writer::save from atomically persisting to the same path. Use a real
    // config file inside a temporary directory, just like production.
    let tmp = tempfile::tempdir().unwrap();
    let cfg = Config {
        server: ServerConfig { bind: "127.0.0.1:0".into(), token: "secret".into(), lang: "en".into() },
        process: vec![], task: vec![], project: vec![],
    };
    let config_path = tmp.path().join("perch.toml");
    perch::config::writer::save(&config_path, &cfg).unwrap();
    let state = AppState::new(cfg, config_path);
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
async fn running_task_can_be_cancelled_without_blocking_manual_api() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "slow", "cmd": "cmd", "args": ["/c", "ping -n 15 127.0.0.1"],
        "schedule": "0 0 0 * * *", "enabled": false, "timeout_secs": 0
    });
    let created: serde_json::Value = client.post(format!("{base}/api/tasks"))
        .bearer_auth(&token).json(&body).send().await.unwrap().json().await.unwrap();
    assert_eq!(created["ok"], true);

    let run: serde_json::Value = client.post(format!("{base}/api/tasks/slow/run"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(run["ok"], true);
    assert!(run["run_id"].as_str().is_some());

    let duplicate: serde_json::Value = client.post(format!("{base}/api/tasks/slow/run"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(duplicate["ok"], false, "duplicate starts must be blocked");

    let stopped: serde_json::Value = client.post(format!("{base}/api/tasks/slow/stop"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(stopped["ok"], true);
    let mut final_state = serde_json::Value::Null;
    for _ in 0..50 {
        let detail: serde_json::Value = client.get(format!("{base}/api/tasks/slow"))
            .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
        if detail["status"] != "running" {
            final_state = detail;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert_eq!(final_state["status"], "cancelled");
    let history: serde_json::Value = client.get(format!("{base}/api/tasks/slow/history"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(history["runs"][0]["status"], "cancelled");
}


async fn await_task_completion(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    name: &str,
) -> serde_json::Value {
    for _ in 0..100 {
        let detail: serde_json::Value = client
            .get(format!("{base}/api/tasks/{name}"))
            .bearer_auth(token)
            .send().await.unwrap().json().await.unwrap();
        if detail["status"] != "running" {
            return detail;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("task {name} did not finish within ten seconds")
}

#[tokio::test]
async fn finished_task_output_history_and_restart_recovery() {
    let (base, token, dir) = spawn_server().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "name": "output_check", "cmd": "cmd",
        "args": ["/c", "echo PERCH_TASK_LOG_471"],
        "schedule": "0 0 0 * * *", "enabled": false
    });
    let created: serde_json::Value = client.post(format!("{base}/api/tasks"))
        .bearer_auth(&token).json(&body).send().await.unwrap().json().await.unwrap();
    assert_eq!(created["ok"], true);
    let started: serde_json::Value = client.post(format!("{base}/api/tasks/output_check/run"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(started["ok"], true);
    let run_id = started["run_id"].as_str().unwrap();

    let done = await_task_completion(&client, &base, &token, "output_check").await;
    assert_eq!(done["status"], "success");
    assert_eq!(done["last_exit_code"], 0);
    assert_eq!(done["run_id"], run_id);

    // Task output is actually persisted and accessible only with API authentication.
    let log_url = format!("{base}/api/tasks/output_check/log/{run_id}");
    let unauthorized = client.get(&log_url).send().await.unwrap();
    assert_eq!(unauthorized.status().as_u16(), 401);
    let log: serde_json::Value = client.get(&log_url).bearer_auth(&token)
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(log["ok"], true);
    assert!(log["content"].as_str().unwrap().contains("PERCH_TASK_LOG_471"));

    let history: serde_json::Value = client.get(format!("{base}/api/tasks/output_check/history"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(history["runs"].as_array().unwrap().len(), 1);
    assert_eq!(history["runs"][0]["status"], "success");
    assert_eq!(history["runs"][0]["run_id"], run_id);

    let cfg_path = dir.path().join("perch.toml");
    let saved = perch::config::load(&cfg_path).unwrap();
    let restored = AppState::new(saved, cfg_path);
    let task = restored.tasks.read().await.get("output_check").cloned().unwrap();
    let snapshot = task.read().await;
    assert_eq!(snapshot.status, "success");
    assert_eq!(snapshot.last_exit_code, Some(0));
    assert_eq!(snapshot.run_id.as_deref(), Some(run_id));
}

#[tokio::test]
async fn failed_exit_and_timeout_are_reported_as_distinct_states() {
    let (base, token, _dir) = spawn_server().await;
    let client = reqwest::Client::new();
    for (name, command, timeout) in [
        ("bad_exit", "exit /b 7", 0_u64),
        ("time_limited", "ping -n 15 127.0.0.1", 1_u64),
    ] {
        let body = serde_json::json!({
            "name": name, "cmd": "cmd", "args": ["/c", command],
            "schedule": "0 0 0 * * *", "enabled": false, "timeout_secs": timeout
        });
        let created: serde_json::Value = client.post(format!("{base}/api/tasks"))
            .bearer_auth(&token).json(&body).send().await.unwrap().json().await.unwrap();
        assert_eq!(created["ok"], true);
        let run: serde_json::Value = client.post(format!("{base}/api/tasks/{name}/run"))
            .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
        assert_eq!(run["ok"], true);
        let final_state = await_task_completion(&client, &base, &token, name).await;
        if name == "bad_exit" {
            assert_eq!(final_state["status"], "failed");
            assert_eq!(final_state["last_exit_code"], 7);
        } else {
            assert_eq!(final_state["status"], "timeout");
            assert!(final_state["last_exit_code"].is_null());
        }
        let history: serde_json::Value = client.get(format!("{base}/api/tasks/{name}/history"))
            .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
        assert_eq!(history["runs"][0]["status"], final_state["status"]);
    }
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


#[tokio::test]
async fn process_working_directory_is_checked_on_the_host() {
    let (base, token, _tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_string_lossy().to_string();
    let file = directory.path().join("config.json");
    std::fs::write(&file, "{}").unwrap();

    let check_url = format!("{base}/api/processes/validate-cwd");
    let without_token = client.post(&check_url)
        .json(&serde_json::json!({ "cwd": path }))
        .send().await.unwrap();
    assert_eq!(without_token.status(), 401);

    let good: serde_json::Value = client.post(&check_url).bearer_auth(&token)
        .json(&serde_json::json!({ "cwd": path }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(good["ok"], true);

    let empty: serde_json::Value = client.post(&check_url).bearer_auth(&token)
        .json(&serde_json::json!({ "cwd": null }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(empty["ok"], true);

    let relative: serde_json::Value = client.post(&check_url).bearer_auth(&token)
        .json(&serde_json::json!({ "cwd": "./relative" }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(relative["error"], "cwd_not_absolute");

    let missing: serde_json::Value = client.post(&check_url).bearer_auth(&token)
        .json(&serde_json::json!({ "cwd": directory.path().join("missing").to_string_lossy() }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(missing["error"], "cwd_not_found");

    let not_directory: serde_json::Value = client.post(&check_url).bearer_auth(&token)
        .json(&serde_json::json!({ "cwd": file.to_string_lossy() }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(not_directory["error"], "cwd_not_directory");
}

#[tokio::test]
async fn cwd_update_preserves_env_restart_and_chinese_alias() {
    let (base, token, cfg_file) = spawn_server().await;
    let client = reqwest::Client::new();
    let workspace = tempfile::tempdir().unwrap();
    let another = tempfile::tempdir().unwrap();
    let cwd = workspace.path().to_string_lossy().to_string();
    let next_cwd = another.path().to_string_lossy().to_string();

    // Bad cwd is rejected before a process is added to the config.
    let rejected: serde_json::Value = client.post(format!("{base}/api/processes"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "name": "bad", "cmd": "cmd",
            "cwd": workspace.path().join("missing").to_string_lossy(),
        }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(rejected["error"], "cwd_not_found");

    let created: serde_json::Value = client.post(format!("{base}/api/processes"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "name": "api_cwd", "cmd": "cmd", "alias_zh": "工作目录",
            "cwd": cwd, "env": { "KEEP_ME": "original" },
            "restart": { "policy": "never", "backoff_secs": 4,
                         "backoff_max_secs": 80, "max_retries": 3 },
        }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(created["ok"], true);

    let changed: serde_json::Value = client.put(format!("{base}/api/processes/api_cwd/cwd"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "cwd": next_cwd }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(changed["ok"], true);

    let details: serde_json::Value = client.get(format!("{base}/api/processes/api_cwd"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(details["cwd"], next_cwd);
    assert_eq!(details["alias_zh"], "工作目录");

    let stored = perch::config::load(&cfg_file.path().join("perch.toml")).unwrap();
    let process = &stored.process[0];
    assert_eq!(process.cwd.as_deref(), Some(next_cwd.as_str()));
    assert_eq!(process.env.get("KEEP_ME").map(String::as_str), Some("original"));
    assert_eq!(process.restart.policy, RestartPolicy::Never);
    assert_eq!(process.restart.max_retries, 3);
    assert_eq!(process.alias_zh.as_deref(), Some("工作目录"));

    // Full modal PUT omits hidden env/restart and must preserve both.
    let full: serde_json::Value = client.put(format!("{base}/api/processes/api_cwd"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "name": "api_cwd", "cmd": "cmd", "args": ["/c", "echo test"],
            "cwd": next_cwd, "alias_zh": "工作目录", "autostart": false
        }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(full["ok"], true);
    let after_full = perch::config::load(&cfg_file.path().join("perch.toml")).unwrap();
    assert_eq!(after_full.process[0].env.get("KEEP_ME").map(String::as_str), Some("original"));
    assert_eq!(after_full.process[0].restart.policy, RestartPolicy::Never);

    let cleared: serde_json::Value = client.put(format!("{base}/api/processes/api_cwd/cwd"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "cwd": null }))
        .send().await.unwrap().json().await.unwrap();
    assert_eq!(cleared["ok"], true);
    let final_cfg = perch::config::load(&cfg_file.path().join("perch.toml")).unwrap();
    assert!(final_cfg.process[0].cwd.is_none());
}


#[tokio::test]
async fn projects_link_existing_tasks_and_show_metadata_without_file_contents() {
    let (base, token, tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let root = tmp.path().join("mock-project");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("accounts.txt"), "DUMMY_PRIVATE_TEST_CONTENT").unwrap();
    std::fs::write(root.join("results.txt"), "MOCK_ONLY").unwrap();
    let job = serde_json::json!({
        "name":"mock_job","cmd":"cmd","args":["/c","echo","MOCK_ONLY"],
        "cwd":root.to_string_lossy(),"schedule":"0 0 0 * * *","enabled":false
    });
    let response: serde_json::Value = client.post(format!("{base}/api/tasks"))
        .bearer_auth(&token).json(&job).send().await.unwrap().json().await.unwrap();
    assert_eq!(response["ok"],true);

    let project = serde_json::json!({
        "name":"Mock Python Project","root":root.to_string_lossy(),
        "tasks":["mock_job"],"inputs":["accounts.txt"],"outputs":["results.txt","not-yet-created.txt"]
    });
    let no_auth = client.get(format!("{base}/api/projects/Mock%20Python%20Project")).send().await.unwrap();
    assert_eq!(no_auth.status(),401);
    let response: serde_json::Value = client.post(format!("{base}/api/projects"))
        .bearer_auth(&token).json(&project).send().await.unwrap().json().await.unwrap();
    assert_eq!(response["ok"],true,response.to_string());

    let view = client.get(format!("{base}/api/projects/Mock%20Python%20Project"))
        .bearer_auth(&token).send().await.unwrap().text().await.unwrap();
    assert!(!view.contains("DUMMY_PRIVATE_TEST_CONTENT"));
    assert!(!view.contains("MOCK_ONLY"),"file contents must not be returned");
    let data: serde_json::Value = serde_json::from_str(&view).unwrap();
    assert_eq!(data["project"]["tasks"][0],"mock_job");
    assert_eq!(data["files"][0]["exists"],true);
    assert_eq!(data["files"][0]["role"],"input");
    assert_eq!(data["files"][0]["size_bytes"],26);
    assert_eq!(data["files"][1]["exists"],true);
    assert_eq!(data["files"][2]["exists"],false);

    let jobs: serde_json::Value = client.get(format!("{base}/api/tasks"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(jobs["tasks"][0]["project"],"Mock Python Project");

    let attempted_delete: serde_json::Value = client.delete(format!("{base}/api/tasks/mock_job"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(attempted_delete["ok"],false);

    let rename = serde_json::json!({
        "name":"changed","cmd":"cmd","schedule":"0 0 0 * * *","enabled":false
    });
    let denied_rename: serde_json::Value = client.put(format!("{base}/api/tasks/mock_job"))
        .bearer_auth(&token).json(&rename).send().await.unwrap().json().await.unwrap();
    assert_eq!(denied_rename["ok"],false);

    let persisted = perch::config::load(&tmp.path().join("perch.toml")).unwrap();
    assert_eq!(persisted.project.len(),1);
    assert_eq!(persisted.project[0].name,"Mock Python Project");

    // Deleting project never removes scripts, existing jobs or result files.
    let deletion: serde_json::Value = client.delete(format!("{base}/api/projects/Mock%20Python%20Project"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(deletion["ok"],true);
    assert!(root.join("results.txt").exists());
    let jobs: serde_json::Value = client.get(format!("{base}/api/tasks"))
        .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(jobs["tasks"][0]["name"],"mock_job");
    assert!(jobs["tasks"][0]["project"].is_null());
}

#[tokio::test]
async fn project_rejects_unsafe_paths_missing_jobs_and_duplicate_membership() {
    let (base, token, tmp) = spawn_server().await;
    let client = reqwest::Client::new();
    let root = tmp.path().to_string_lossy().to_string();
    let bad = serde_json::json!({
        "name":"bad","root":root,"tasks":[],"inputs":["../secret.txt"],"outputs":[]
    });
    let result: serde_json::Value = client.post(format!("{base}/api/projects"))
        .bearer_auth(&token).json(&bad).send().await.unwrap().json().await.unwrap();
    assert_eq!(result["ok"],false,result.to_string());
    let missing_job = serde_json::json!({
        "name":"bad","root":root,"tasks":["not-a-task"],"inputs":[],"outputs":[]
    });
    let result: serde_json::Value = client.post(format!("{base}/api/projects"))
        .bearer_auth(&token).json(&missing_job).send().await.unwrap().json().await.unwrap();
    assert_eq!(result["ok"],false);

    let project = serde_json::json!({
        "name":"Valid","root":root,"tasks":[],"inputs":["input.txt"],"outputs":[]
    });
    let result: serde_json::Value = client.post(format!("{base}/api/projects"))
        .bearer_auth(&token).json(&project).send().await.unwrap().json().await.unwrap();
    assert_eq!(result["ok"],true);
    let duplicate = client.post(format!("{base}/api/projects"))
        .bearer_auth(&token).json(&project).send().await.unwrap().json().await.unwrap();
    let duplicate: serde_json::Value = duplicate;
    assert_eq!(duplicate["ok"],false);
}
