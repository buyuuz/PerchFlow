//! A Docker CLI adapter. No shell, no Docker daemon changes, no extra service.
use axum::{extract::{Path,Query},routing::{get,post},Json,Router};
use chrono::{DateTime,Utc};
use serde::Deserialize;
use serde_json::{json,Value};
use std::{path::PathBuf,sync::OnceLock,time::Duration};
use tokio::{process::Command,sync::Mutex,time::timeout};
use crate::web::WebContext;

pub fn routes()->Router<WebContext>{
 Router::new().route("/docker/environment",get(environment))
 .route("/docker/containers",get(containers))
 .route("/docker/containers/:id",get(detail))
 .route("/docker/containers/:id/logs",get(logs))
 .route("/docker/containers/:id/start",post(start))
 .route("/docker/containers/:id/stop",post(stop))
 .route("/docker/containers/:id/restart",post(restart))
}
#[derive(Debug)] struct DockerError{code:&'static str,message:String}
type Result<T>=std::result::Result<T,DockerError>;
fn error(code:&'static str,message:impl Into<String>)->DockerError{DockerError{code,message:message.into()}}
fn failed(e:DockerError)->Value{json!({"ok":false,"connected":!matches!(e.code,"cli_not_found"|"engine_unavailable"|"remote_engine_blocked"),"error":e.code,"message":e.message})}
fn cli_path()->Result<PathBuf>{
 for key in ["PERCHFLOW_DOCKER_CLI","DOCKER_CLI_PATH"]{
   if let Some(v)=std::env::var_os(key){let p=PathBuf::from(v);
     return if p.is_absolute()&&p.is_file(){Ok(p)}else{Err(error("cli_not_found",format!("{key} must be an existing absolute file")))}
   }
 }
 #[cfg(windows)] for p in [r"C:\Program Files\Docker\Docker\resources\bin\docker.exe",r"C:\Program Files\Docker\docker.exe"]{
   let path=PathBuf::from(p);if path.is_file(){return Ok(path)}
 }
 if let Some(paths)=std::env::var_os("PATH"){
   for p in std::env::split_paths(&paths){
     let exe=p.join(if cfg!(windows){"docker.exe"}else{"docker"});
     if exe.is_file(){return Ok(exe)}
   }
 }
 Err(error("cli_not_found","Docker CLI not found for this Windows account; check PATH or PERCHFLOW_DOCKER_CLI"))
}
async fn cli(path:&PathBuf,args:&[&str],secs:u64)->Result<(String,String)>{
 let mut c=Command::new(path);c.args(args).kill_on_drop(true).stdin(std::process::Stdio::null());
 let o=timeout(Duration::from_secs(secs),c.output()).await
   .map_err(|_|error("timeout",format!("Docker command exceeded {secs} seconds")))?
   .map_err(|e|error("cli_error",format!("Cannot launch Docker CLI: {e}")))?;
 let stdout=String::from_utf8_lossy(&o.stdout).into_owned();
 let stderr=String::from_utf8_lossy(&o.stderr).into_owned();
 if !o.status.success(){return Err(error("docker_error",format!("Docker exited {:?}: {}",o.status.code(),stderr.trim()).chars().take(1100).collect::<String>()))}
 Ok((stdout,stderr))
}
fn local_endpoint(host:&str)->bool{
 let host=host.trim().trim_matches('"').to_ascii_lowercase();
 host.starts_with("npipe:////./pipe/") || host.starts_with("npipe://./pipe/") ||
 host.starts_with("tcp://127.0.0.1:") || host.starts_with("tcp://localhost:") ||
 host.starts_with("tcp://[::1]:") || host=="unix:///var/run/docker.sock"
}
async fn engine()->Result<PathBuf>{
 let path=cli_path()?;
 // Docker CLI uses contexts and DOCKER_HOST, which may point to remote hosts.
 // V1 must never operate on a remote Docker daemon.
 if let Ok(v)=std::env::var("DOCKER_HOST"){
  if !v.trim().is_empty() && !local_endpoint(&v){
   return Err(error("remote_engine_blocked","Remote DOCKER_HOST is not supported; only local Docker Engine can be managed"));
  }
 }
 let endpoint=cli(&path,&["context","inspect","--format","{{.Endpoints.docker.Host}}"],6)
     .await.map_err(|e|error("engine_unavailable",format!("Cannot read current Docker context: {}",e.message)))?.0;
 if !local_endpoint(&endpoint){
  return Err(error("remote_engine_blocked",format!("Remote Docker context refused (endpoint: {})",endpoint.trim())));
 }
 match cli(&path,&["info","--format","{{.ServerVersion}}"],7).await{
  Ok((version,_)) if !version.trim().is_empty()=>Ok(path),
  Ok(_)=>Err(error("engine_unavailable","Docker Engine did not return a server version")),
  Err(e)=>Err(error("engine_unavailable",format!("Docker Engine unavailable for this Windows account: {}",e.message))),
 }
}
async fn environment()->Json<Value>{
 let response=match cli_path(){
  Err(e)=>failed(e),
  Ok(path)=>{
   let version=cli(&path,&["--version"],5).await.ok().map(|v|v.0.trim().to_string());
   match engine().await{
    Ok(_)=>json!({"ok":true,"connected":true,"cli_path":path.to_string_lossy(),"cli_version":version}),
    Err(e)=>{let mut r=failed(e);r["cli_path"]=json!(path.to_string_lossy().to_string());r["cli_version"]=json!(version);r}
   }
  }
 };
 Json(response)
}
fn lines(raw:&str)->Result<Vec<Value>>{
 raw.lines().filter(|s|!s.trim().is_empty()).map(|s|serde_json::from_str(s).map_err(|e|error("parse_error",format!("Invalid Docker JSON output: {e}")))).collect()
}
fn id_ok(id:&str)->bool{id.len()==64&&id.bytes().all(|b|b.is_ascii_hexdigit())}
fn parse_mem(s:&str)->Option<u64>{
 let s=s.split('/').next()?.trim();
 let i=s.find(|c:char|!c.is_ascii_digit()&&c!='.')?;
 let (amount,unit)=s.split_at(i);
 let mult=match unit.trim(){"B"=>1.,"kB"|"KB"=>1000.,"MB"=>1e6,"GB"=>1e9,"KiB"=>1024.,"MiB"=>1048576.,"GiB"=>1073741824.,"TiB"=>1099511627776.,_=>return None};
 let n=amount.parse::<f64>().ok()?*mult;
 (n.is_finite()&&n>=0.&&n<=u64::MAX as f64).then_some(n as u64)
}
fn cpu(s:&str)->Option<f64>{s.trim().trim_end_matches('%').parse::<f64>().ok().filter(|n|n.is_finite()&&*n>=0.)}
fn status(i:&Value)->&str{i["State"]["Status"].as_str().unwrap_or("unknown")}
fn uptime(started:&str,state:&str)->Option<i64>{
 if state!="running"{return None}
 let dt=DateTime::parse_from_rfc3339(started).ok()?;
 Some((Utc::now()-dt.with_timezone(&Utc)).num_seconds().max(0))
}
async fn batch_inspect(path:&PathBuf,ids:&[String])->Result<Vec<Value>>{
 let mut out=Vec::new();
 for part in ids.chunks(24){
  let mut args=vec!["inspect"];args.extend(part.iter().map(String::as_str));
  let (raw,_)=cli(path,&args,12).await?;
  out.extend(serde_json::from_str::<Vec<Value>>(&raw).map_err(|e|error("parse_error",format!("inspect JSON invalid: {e}")))?);
 }
 Ok(out)
}
async fn collect(path:&PathBuf)->Result<Vec<Value>>{
 let (raw,_)=cli(path,&["ps","-a","--no-trunc","--format","{{json .}}"],9).await?;
 let rows=lines(&raw)?;
 let mut ids=Vec::new();
 for row in &rows{
  let id=row["ID"].as_str().unwrap_or("");
  if !id_ok(id){return Err(error("parse_error","docker ps returned a non-full container ID"))}
  ids.push(id.to_string());
 }
 if ids.is_empty(){return Ok(vec![])}
 let info=batch_inspect(path,&ids).await?;
 let details:std::collections::HashMap<String,Value>=info.into_iter()
   .filter_map(|v|v["Id"].as_str().map(|id|(id.to_lowercase(),v.clone()))).collect();
 let running:Vec<_>=ids.iter().filter(|id|details.get(&id.to_lowercase())
    .and_then(|v|v["State"]["Running"].as_bool()).unwrap_or(false)).collect();
 let mut stats=std::collections::HashMap::<String,Value>::new();
 for group in running.chunks(24){
  let mut args=vec!["stats","--no-stream","--format","{{json .}}"];
  args.extend(group.iter().map(|s|s.as_str()));
  // Engine stats are best-effort; their failure does not falsify container state.
  if let Ok((output,_))=cli(path,&args,11).await{
   if let Ok(values)=lines(&output){for value in values{
    if let Some(partial)=value["ID"].as_str(){
      if let Some(full)=ids.iter().find(|id|id.starts_with(partial)){stats.insert(full.to_string(),value);}
    }
   }}
  }
 }
 let mut result=Vec::new();
 for row in rows{
  let id=row["ID"].as_str().unwrap_or("");
  let i=details.get(&id.to_lowercase()).ok_or_else(||error("inspect_error","Container changed during polling; refresh again"))?;
  let state=status(i);
  let started=i["State"]["StartedAt"].as_str().filter(|s|!s.starts_with("0001-")).unwrap_or("");
  let stat=stats.get(id);
  let mem=stat.and_then(|s|s["MemUsage"].as_str()).unwrap_or("");
  let bytes=parse_mem(mem);
  result.push(json!({
    "id":id,"short_id":id.chars().take(12).collect::<String>(),
    "name":i["Name"].as_str().unwrap_or("").trim_start_matches('/'),
    "image":row["Image"].as_str().unwrap_or(""),
    "status":state,"status_text":row["Status"].as_str().unwrap_or(""),
    "cpu_percent":stat.and_then(|v|v["CPUPerc"].as_str()).and_then(cpu),
    "memory_bytes":bytes,"memory_mib":bytes.map(|n|n as f64/1048576.),
    "memory_text":if mem.is_empty(){None}else{Some(mem)},
    "started_at":if started.is_empty(){None}else{Some(started)},
    "uptime_seconds":uptime(started,state),
    "restart_count":i["RestartCount"].as_i64().unwrap_or(0),
    "exit_code":i["State"]["ExitCode"].as_i64().unwrap_or(0)
  }));
 }
 Ok(result)
}
async fn containers()->Json<Value>{
 let out=match engine().await{
  Err(e)=>failed(e),
  Ok(path)=>match collect(&path).await{
   Ok(cs)=>json!({"ok":true,"connected":true,"containers":cs,"updated_at":Utc::now().to_rfc3339()}),
   Err(e)=>failed(e)
  }
 };
 Json(out)
}
async fn inspect_one(path:&PathBuf,id:&str)->Result<Value>{
 if !id_ok(id){return Err(error("invalid_id","Full 64-character Docker ID required"))}
 let (raw,_)=cli(path,&["inspect",id],9).await?;
 let mut arr=serde_json::from_str::<Vec<Value>>(&raw).map_err(|e|error("parse_error",format!("Bad inspect output: {e}")))?;
 let v=arr.pop().ok_or_else(||error("not_found","Container no longer exists"))?;
 if v["Id"].as_str()!=Some(id){return Err(error("not_found","Container ID changed"))}
 Ok(v)
}
async fn detail(Path(id):Path<String>)->Json<Value>{
 let response=async{
  let path=engine().await?;let i=inspect_one(&path,&id).await?;
  let state=status(&i);let started=i["State"]["StartedAt"].as_str().unwrap_or("");
  Ok::<_,DockerError>(json!({"ok":true,"connected":true,"container":{
   "id":id,"name":i["Name"].as_str().unwrap_or("").trim_start_matches('/'),
   "image":i["Config"]["Image"].as_str().unwrap_or(""),
   "status":state,"running":i["State"]["Running"].as_bool().unwrap_or(false),
   "started_at":started,"created_at":i["Created"].as_str().unwrap_or(""),
   "uptime_seconds":uptime(started,state),
   "restart_count":i["RestartCount"].as_i64().unwrap_or(0),
   "exit_code":i["State"]["ExitCode"].as_i64().unwrap_or(0)
  }}))
 }.await;
 Json(response.unwrap_or_else(failed))
}
#[derive(Deserialize)]struct LogOptions{tail:Option<u32>}
async fn logs(Path(id):Path<String>,Query(q):Query<LogOptions>)->Json<Value>{
 let response=async{
  let path=engine().await?;inspect_one(&path,&id).await?;
  let n=q.tail.unwrap_or(200).clamp(1,1000).to_string();
  let (out,err)=cli(&path,&["logs","--timestamps","--tail",&n,&id],12).await?;
  // On Docker, container stderr is emitted via CLI stderr on successful exit.
  let mut chunks=out.lines().chain(err.lines()).collect::<Vec<_>>();
  chunks.sort_unstable(); // timestamps preserve original cross-stream order
  let text=chunks.join("\n");
  let limited=text.chars().take(250000).collect::<String>();
  Ok::<_,DockerError>(json!({"ok":true,"logs":limited,
     "truncated":text.chars().count()>250000,"tail":n}))
 }.await;
 Json(response.unwrap_or_else(failed))
}
static OPERATIONS:OnceLock<Mutex<()>>=OnceLock::new();
async fn operation(id:String,kind:&'static str)->Json<Value>{
 if !id_ok(&id){return Json(failed(error("invalid_id","Invalid full container ID")))}
 let _guard=OPERATIONS.get_or_init(||Mutex::new(())).lock().await;
 let response=async{
  let path=engine().await?;
  let before=inspect_one(&path,&id).await?;
  let state=status(&before);
  if kind=="start"&&state=="running"{return Err(error("invalid_state","Container already running"))}
  if kind=="stop"&&state!="running"&&state!="restarting"{return Err(error("invalid_state","Container not running"))}
  let args=match kind{
   "start"=>vec!["start",id.as_str()],
   "stop"=>vec!["stop","--time","10",id.as_str()],
   "restart"=>vec!["restart","--time","10",id.as_str()],
   _=>return Err(error("invalid_action","Unsupported Docker operation"))
  };
  cli(&path,&args,25).await?;
  let after=inspect_one(&path,&id).await?;
  Ok::<_,DockerError>(json!({"ok":true,"id":id,"action":kind,"status":status(&after),
       "running":after["State"]["Running"].as_bool().unwrap_or(false)}))
 }.await;
 Json(response.unwrap_or_else(failed))
}
async fn start(Path(id):Path<String>)->Json<Value>{operation(id,"start").await}
async fn stop(Path(id):Path<String>)->Json<Value>{operation(id,"stop").await}
async fn restart(Path(id):Path<String>)->Json<Value>{operation(id,"restart").await}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn safe_ids(){assert!(id_ok(&"a".repeat(64)));assert!(!id_ok("mysql"));assert!(!id_ok(&"a".repeat(12)));assert!(!id_ok(&format!("{};","a".repeat(63))))}
 #[test]fn resource_units(){assert_eq!(parse_mem("100.8MiB / 2GiB"),Some(105696460));assert_eq!(parse_mem("1.5GiB"),Some(1610612736));assert_eq!(cpu("0.20%"),Some(0.2));assert_eq!(cpu("—"),None)}
 #[test]fn exited_not_running(){assert!(uptime("2026-10-20T00:00:00Z","exited").is_none())}
 #[test]fn local_context_only(){assert!(local_endpoint("npipe:////./pipe/docker_engine"));assert!(local_endpoint("tcp://127.0.0.1:2375"));assert!(!local_endpoint("ssh://root@server"));assert!(!local_endpoint("tcp://192.168.1.2:2375"));}
}
