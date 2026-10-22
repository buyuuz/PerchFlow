// Independent fake Docker CLI. Never touches Docker Engine.
// Built only by the isolated Docker V1 CI workflow.
use std::{env,fs,process};
const A:&str="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B:&str="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
fn main(){
 let args:Vec<String>=env::args().skip(1).collect();
 let cmd=args.first().map(String::as_str).unwrap_or("");
 let file=env::var("PF_DOCKER_MOCK_STATE").expect("mock state file");
 let running=fs::read_to_string(&file).unwrap_or("running".into()).trim()=="running";
 match cmd{
  "--version"=>println!("Docker version 27.1.0, build mocked"),
  "context"=>println!("npipe:////./pipe/docker_engine"),
  "info"=>{
   if env::var("PF_DOCKER_MOCK_DISCONNECTED").ok().as_deref()==Some("1"){
    eprintln!("error during connect: open //./pipe/docker_engine: access is denied");process::exit(1)
   }
   println!("27.1.0");
  }
  "ps"=>{
   println!("{{\"ID\":\"{}\",\"Names\":\"example-worker\",\"Image\":\"example:latest\",\"State\":\"{}\",\"Status\":\"{}\"}}",A,
    if running{"running"}else{"exited"},if running{"Up 1 hour"}else{"Exited (0)"});
   println!("{{\"ID\":\"{}\",\"Names\":\"example-db\",\"Image\":\"postgres:16\",\"State\":\"exited\",\"Status\":\"Exited (1)\"}}",B);
  }
  "inspect"=>{
   let ids=args.iter().skip(1).filter(|s|s.len()==64).collect::<Vec<_>>();
   if ids.iter().any(|s|*s!=A && *s!=B) || ids.is_empty(){eprintln!("Error: No such container");process::exit(1)}
   println!("[{}]",ids.iter().map(|id|{
    let is_a=id.as_str()==A;let on=is_a&&running;
    let state=if on{"running"}else{"exited"};
    format!("{{\"Id\":\"{}\",\"Name\":\"/{}\",\"Created\":\"2026-10-20T00:00:00Z\",\"Config\":{{\"Image\":\"{}\"}},\"RestartCount\":{},\"State\":{{\"Status\":\"{}\",\"Running\":{},\"ExitCode\":{},\"StartedAt\":\"2026-10-20T08:00:00Z\"}}}}",
      id,if is_a{"example-worker"}else{"example-db"},if is_a{"example:latest"}else{"postgres:16"},
      if is_a{2}else{0},state,on,if is_a{0}else{1})
   }).collect::<Vec<_>>().join(","));
  }
  "stats"=>{
    if running {println!("{{\"ID\":\"{}\",\"Name\":\"example-worker\",\"CPUPerc\":\"0.20%\",\"MemUsage\":\"100.8MiB / 2GiB\"}}",A)}
  }
  "logs"=>{
   if let Ok(path)=env::var("PF_DOCKER_MOCK_LARGE_FILE"){
     if fs::read_to_string(path).ok().as_deref()==Some("1"){
       println!("{}", "x".repeat(700_000));
       return;
     }
   }
   println!("2026-10-22T08:00:00Z mock STDOUT log");
   eprintln!("2026-10-22T08:00:01Z mock STDERR log");
  }
  "start"|"stop"|"restart"=>{
   let id=args.last().map(String::as_str).unwrap_or("");
   if id!=A{eprintln!("operation refused by test fixture");process::exit(1)}
   let next=if cmd=="stop"{"exited"}else{"running"};
   fs::write(&file,next).expect("write fake status");
   println!("{}",id);
  }
  _=>{eprintln!("unsupported mock Docker CLI arguments: {:?}",args);process::exit(2)}
 }
}
