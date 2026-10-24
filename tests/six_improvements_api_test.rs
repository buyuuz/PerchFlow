use perch::config::model::*;
use perch::core::{scheduler::Scheduler, state::AppState, supervisor::Supervisor};
use perch::web::{build_router,WebContext};
use serde_json::{json,Value};
async fn mock_server() -> (String,String,tempfile::TempDir) {
  let tmp=tempfile::tempdir().unwrap();
  let cfg=Config{server:ServerConfig{bind:"127.0.0.1:0".into(),token:"token".into(),lang:"en".into()},process:vec![],task:vec![],project:vec![]};
  let path=tmp.path().join("perch.toml");
  perch::config::writer::save(&path,&cfg).unwrap();
  let state=AppState::new(cfg,path);
  let ctx=WebContext{supervisor:Supervisor::new(state.clone()),scheduler:Scheduler::new(state.clone()),state};
  let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr=listener.local_addr().unwrap();
  tokio::spawn(async move{axum::serve(listener,build_router(ctx)).await.unwrap()});
  (format!("http://{addr}"),"token".into(),tmp)
}
async fn post(client:&reqwest::Client,base:&str,token:&str,path:&str,body:Value)->Value{
  client.post(format!("{base}{path}")).bearer_auth(token).json(&body).send().await.unwrap().json().await.unwrap()
}
#[tokio::test]
async fn cross_project_duplicate_labels_do_not_collide() {
  let (base,token,tmp)=mock_server().await;
  let client=reqwest::Client::new();
  let exe=std::env::current_exe().unwrap();
  let mut keys=Vec::new();
  for name in ["ProjectA","ProjectB"] {
    let root=tmp.path().join(name);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("sync.py"),"print('ok')").unwrap();
    let p=post(&client,&base,&token,"/api/projects",json!({"name":name,"root":root.to_string_lossy()})).await;
    assert_eq!(p["ok"],true,"{p}");
    let task=post(&client,&base,&token,&format!("/api/projects/{name}/python-task"),
      json!({"name":"Data Sync","python":exe.to_string_lossy(),"script":"sync.py"})).await;
    assert_eq!(task["ok"],true,"{task}");
    let key=task["task"].as_str().unwrap().to_owned();
    let detail:Value=client.get(format!("{base}/api/tasks/{key}")).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
    assert_eq!(detail["project"],name);
    assert_eq!(detail["display_name"].as_str().unwrap_or(&key),"Data Sync");
    keys.push(key);
  }
  assert_ne!(keys[0],keys[1]);
  let root=tmp.path().join("ProjectA");
  let duplicate=post(&client,&base,&token,"/api/projects/ProjectA/python-task",
    json!({"name":"Data Sync","python":exe.to_string_lossy(),"script":"sync.py"})).await;
  assert_eq!(duplicate["ok"],false,"{duplicate}");
  let schedule:Value=client.put(format!("{base}/api/tasks/{}/schedule",keys[0]))
    .bearer_auth(&token).json(&json!({"schedule":"0 0 9 * * *","enabled":false}))
    .send().await.unwrap().json().await.unwrap();
  assert_eq!(schedule["ok"],true,"{schedule}");
  let other:Value=client.get(format!("{base}/api/tasks/{}",keys[1])).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
  assert_eq!(other["schedule"],"");
  let deletion:Value=client.delete(format!("{base}/api/projects/ProjectA/tasks/{}",keys[0]))
    .bearer_auth(&token).send().await.unwrap().json().await.unwrap();
  assert_eq!(deletion["ok"],true,"{deletion}");
  let untouched:Value=client.get(format!("{base}/api/tasks/{}",keys[1])).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
  assert_eq!(untouched["project"],"ProjectB");
  let persisted=perch::config::load(&tmp.path().join("perch.toml")).unwrap();
  assert_eq!(persisted.task.len(),1);
  assert_eq!(persisted.task[0].name,keys[1]);
  assert!(root.join("sync.py").exists(),"removing task must preserve source files");
}
#[tokio::test]
async fn process_advanced_settings_preserved_by_base_editor() {
  let (base,token,_tmp)=mock_server().await;
  let client=reqwest::Client::new();
  let value=post(&client,&base,&token,"/api/processes",json!({
    "name":"MailHub","cmd":"cmd","args":[],"autostart":false,
    "env":{"MAILHUB_PROVIDER_PROXY":"http://127.0.0.1:7897"},
    "restart":{"policy":"on-failure","backoff_secs":2,"backoff_max_secs":60,"max_retries":0}
  })).await;
  assert_eq!(value["ok"],true,"{value}");
  let list:Value=client.get(format!("{base}/api/processes")).bearer_auth(&token).send().await.unwrap().json().await.unwrap();
  assert!(list["processes"][0].get("env").is_none(),"polling should not expose env");
  let url:Value=client.put(format!("{base}/api/processes/MailHub/web-url")).bearer_auth(&token)
    .json(&json!({"web_url":"http://127.0.0.1:9877"}))
    .send().await.unwrap().json().await.unwrap();
  assert_eq!(url["ok"],true,"{url}");
  let updated:Value=client.put(format!("{base}/api/processes/MailHub")).bearer_auth(&token)
    .json(&json!({"name":"MailHub","cmd":"cmd","args":[],"autostart":false,"alias_zh":"邮箱管理中心"}))
    .send().await.unwrap().json().await.unwrap();
  assert_eq!(updated["ok"],true,"{updated}");
  let settings:Value=client.get(format!("{base}/api/processes/MailHub/settings")).bearer_auth(&token)
    .send().await.unwrap().json().await.unwrap();
  assert_eq!(settings["env"]["MAILHUB_PROVIDER_PROXY"],"http://127.0.0.1:7897");
  assert_eq!(settings["restart"]["policy"],"on-failure");
  assert_eq!(settings["web_url"],"http://127.0.0.1:9877");
}
