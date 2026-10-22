#!/usr/bin/env python3
"""Windows Docker V1 integration with fake Docker CLI. No real Docker operations."""
import argparse, json, os, shutil, socket, subprocess, tempfile, time, urllib.error, urllib.request
from pathlib import Path

parser=argparse.ArgumentParser()
parser.add_argument("--exe",required=True)
parser.add_argument("--mock",required=True)
args=parser.parse_args()
exe=Path(args.exe).resolve()
mock=Path(args.mock).resolve()
A="a"*64;B="b"*64
with tempfile.TemporaryDirectory(prefix="perch-docker-v1-") as tmp:
    root=Path(tmp)
    shutil.copy2(exe, root / "perch.exe")
    state=root / "fake_state.txt"
    state.write_text("running")
    oversized=root / "mock_large_logs.txt"
    oversized.write_text("0")
    with socket.socket() as sock:
        sock.bind(("127.0.0.1",0))
        port=sock.getsockname()[1]
    token="docker-v1-isolated-test-token"
    (root/"perch.toml").write_text(f'[server]\nbind = "127.0.0.1:{port}"\ntoken = "{token}"\nlang = "zh"\n',encoding="utf-8")
    env=os.environ.copy()
    env["PERCHFLOW_DOCKER_CLI"]=str(mock)
    env["PF_DOCKER_MOCK_STATE"]=str(state)
    env["PF_DOCKER_MOCK_LARGE_FILE"]=str(oversized)
    proc=subprocess.Popen([str(root/"perch.exe"),"run"],cwd=root,env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    def req(route,method="GET",auth=True):
        headers={"Authorization":f"Bearer {token}"} if auth else {}
        request=urllib.request.Request(f"http://127.0.0.1:{port}{route}",headers=headers,method=method)
        try:
            with urllib.request.urlopen(request,timeout=25) as response:
                body=response.read().decode("utf-8")
                if route.startswith("/api/"): return response.status,json.loads(body)
                return response.status,body
        except urllib.error.HTTPError as error:
            return error.code,error.read().decode("utf-8")
    try:
        deadline=time.time()+16
        while time.time()<deadline:
            if proc.poll() is not None: raise AssertionError("PerchFlow exited early")
            try:
                if req("/docker")[0]==200:break
            except (OSError,TimeoutError):
                time.sleep(.2)
        else: raise AssertionError("PerchFlow did not start")
        code,body=req("/api/docker/containers",auth=False)
        assert code==401,(code,body)
        code,body=req("/api/docker/environment")
        assert code==200 and body["ok"] and body["connected"],body
        code,body=req("/api/docker/containers")
        assert body["ok"] and len(body["containers"])==2,body
        worker,db=body["containers"]
        assert worker["id"]==A and worker["name"]=="example-worker" and worker["status"]=="running",worker
        assert abs(worker["cpu_percent"]-0.2)<.000001 and abs(worker["memory_mib"]-100.8)<.01,worker
        assert db["id"]==B and db["status"]=="exited" and db["cpu_percent"] is None,db
        code,body=req("/api/docker/containers/"+A)
        assert body["ok"] and body["container"]["restart_count"]==2,body
        code,body=req("/api/docker/containers/"+A+"/logs?tail=200")
        assert body["ok"] and "STDOUT" in body["logs"] and "STDERR" in body["logs"],body
        oversized.write_text("1")
        code,body=req("/api/docker/containers/"+A+"/logs?tail=1000")
        assert body["ok"] is False and body["error"]=="output_too_large",body
        oversized.write_text("0")
        code,body=req("/api/docker/containers/not-a-real-id/stop","POST")
        assert body["ok"] is False and body["error"]=="invalid_id",body
        code,body=req("/api/docker/containers/"+A+"/stop","POST")
        assert body["ok"] and body["status"]=="exited",body
        code,body=req("/api/docker/containers")
        assert body["containers"][0]["status"]=="exited" and body["containers"][0]["cpu_percent"] is None,body
        code,body=req("/api/docker/containers/"+A+"/start","POST")
        assert body["ok"] and body["status"]=="running",body
        code,body=req("/api/docker/containers/"+A+"/restart","POST")
        assert body["ok"] and body["status"]=="running",body
        code,body=req("/api/docker/containers/"+B+"/stop","POST")
        assert body["ok"] is False and body["error"]=="invalid_state",body
        # Docker disconnect is not equivalent to all containers having stopped.
        proc.terminate();proc.wait(timeout=10);proc=None
        env["DOCKER_HOST"]="tcp://192.168.50.10:2375"
        proc=subprocess.Popen([str(root/"perch.exe"),"run"],cwd=root,env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        deadline=time.time()+12
        while time.time()<deadline:
            try:
                code,body=req("/api/docker/containers")
                if code==200 and not body["ok"]:break
            except (OSError,TimeoutError):time.sleep(.2)
        else:raise AssertionError("Remote Docker host not blocked")
        assert body["error"]=="remote_engine_blocked" and body["connected"] is False,body
        proc.terminate();proc.wait(timeout=10);proc=None
        env.pop("DOCKER_HOST",None)
        env["PF_DOCKER_MOCK_DISCONNECTED"]="1"
        proc=subprocess.Popen([str(root/"perch.exe"),"run"],cwd=root,env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        deadline=time.time()+12
        while time.time()<deadline:
            try:
                code,body=req("/api/docker/containers")
                if code==200 and not body["ok"]:break
            except (OSError,TimeoutError):time.sleep(.2)
        else:raise AssertionError("Disconnected Engine not detected")
        assert body["connected"] is False and body["error"]=="engine_unavailable" and "containers" not in body,body
        print("DOCKER_V1_INTEGRATION_OK: env, auth, ps-a, running+stopped, stats, inspect, logs stdout+stderr, stop/start/restart, invalid ID, disconnected engine")
    finally:
        if proc and proc.poll() is None:
            proc.terminate()
            try:proc.wait(timeout=8)
            except subprocess.TimeoutExpired:proc.kill();proc.wait()
