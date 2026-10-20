"""Project Hub V2 isolated Edge smoke test on GitHub-hosted Windows only.

All API data is fake. No real PerchFlow service, automation script, secrets,
Firecrawl login, live network operation, or device connection is used.
"""
import json
import os
from pathlib import Path
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
import threading
import urllib.parse

from playwright.sync_api import sync_playwright, expect

ROOT = Path(__file__).resolve().parents[1] / "frontend"
TOKEN = "MOCK_ONLY_TOKEN"
PROJECT_NAME = "Mock Project"
PROJECT = {
    "name": PROJECT_NAME, "root": r"C:\CI\MOCK_PROJECT",
    "tasks": ["mock_job"], "inputs": ["accounts.txt"], "outputs": ["results.txt"],
    "output_dirs": ["reports"],
    "preflight_inputs": True,
}
TASKS = [{"name":"mock_job", "cmd":"cmd", "args":["/c","echo MOCK_ONLY"],
          "cwd":PROJECT["root"], "schedule":"0 0 0 * * *",
          "enabled":False, "status":"success", "project":PROJECT_NAME}]
FILES = [
    {"role":"input","path":"accounts.txt","exists":True,"size_bytes":25,"modified_unix":1792500000},
    {"role":"output","path":"results.txt","exists":True,"size_bytes":15,"modified_unix":1792500000},
    {"role":"output_dir","path":"reports","exists":True,"is_directory":True,"size_bytes":None,"modified_unix":1792500000},
]
CHANGES = {
    "project":PROJECT_NAME,"task":"mock_job","run_id":"123-1-1",
    "scanned_paths":3,
    "incomplete_scan":False,
    "changes":[{
        "role":"output","path":"reports\\generated.xlsx","change":"created",
        "before":{"exists":False},"after":{"exists":True,"size_bytes":15,"discovered":True}
    }]
}
CAPTURED = []
LOG_READS = []

def check_ci():
    assert os.name == "nt" and os.environ.get("GITHUB_ACTIONS") == "true"
    assert os.environ.get("RUNNER_OS") == "Windows"
    assert os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"

class Handler(SimpleHTTPRequestHandler):
    def __init__(self,*args,**kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)
    def log_message(self,fmt,*args):
        pass
    def api(self, status, data):
        body=json.dumps(data).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type","application/json")
        self.send_header("Content-Length",str(len(body)))
        self.end_headers()
        self.wfile.write(body)
    def authenticate(self):
        if self.headers.get("Authorization") != "Bearer " + TOKEN:
            self.api(401, {"ok":False,"error":"unauthorized"})
            return False
        return True
    def do_GET(self):
        u=urllib.parse.urlsplit(self.path)
        path=urllib.parse.unquote(u.path)
        if path.startswith("/api/"):
            if not self.authenticate():
                return
            if path=="/api/projects": return self.api(200,{"projects":[PROJECT]})
            if path=="/api/tasks": return self.api(200,{"tasks":TASKS})
            if path==f"/api/projects/{PROJECT_NAME}":
                return self.api(200,{"ok":True,"project":PROJECT,"files":FILES})
            if path==f"/api/projects/{PROJECT_NAME}/preflight":
                return self.api(200,{"ok":True,"preflight":{
                    "enabled":True,"checked":1,"ready":True,"unavailable":[],"reason":None
                }})
            if path=="/api/tasks/mock_job/history":
                return self.api(200,{"ok":True,"runs":[{
                    "run_id":"123-1-1","started_at":"2026-10-20T00:00:00Z",
                    "status":"success","files_recorded":True,"duration_ms":1234,"exit_code":0,
                    "preflight":{"enabled":True,"ready":True,"checked":1,"unavailable":[]},
                    "output_summary":{"created":1,"modified":0,"removed":0,"incomplete_scan":False}
                }]})
            if path=="/api/tasks/mock_job/files/123-1-1":
                return self.api(200,{"ok":True,"record":CHANGES})
            if path=="/api/tasks/mock_job/log/123-1-1":
                LOG_READS.append(path)
                return self.api(200,{"ok":True,"content":"MOCK_LOG_ONLY","truncated":False})
            return self.api(404,{"ok":False,"error":"not found"})
        if path=="/projects":
            self.path="/projects.html"
        return super().do_GET()
    def do_POST(self):
        path=urllib.parse.unquote(urllib.parse.urlsplit(self.path).path)
        if not self.authenticate():
            return
        length=int(self.headers.get("Content-Length",0))
        data=json.loads(self.rfile.read(length).decode("utf-8"))
        if path==f"/api/projects/{PROJECT_NAME}/python-task":
            CAPTURED.append(data)
            assert data["name"] == "mock_python", data
            assert data["script"] == "mock_worker.py", data
            assert data["args"] == ["--limit","10"], data
            PROJECT["tasks"].append("mock_python")
            PROJECT["inputs"].append("mock_worker.py")
            TASKS.append({"name":"mock_python","cmd":data["python"],"args":[data["script"]],
                "cwd":PROJECT["root"],"schedule":data["schedule"],"enabled":False,
                "project":PROJECT_NAME,"status":"idle"})
            FILES.append({"role":"input","path":"mock_worker.py","exists":True,"size_bytes":30,"modified_unix":1792500000})
            return self.api(200,{"ok":True,"task":"mock_python","enabled":False})
        return self.api(404,{"ok":False,"error":"not found"})

def main():
    check_ci()
    server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
    thread=threading.Thread(target=server.serve_forever,daemon=True)
    thread.start()
    target=Path("dev/reports")
    target.mkdir(parents=True,exist_ok=True)
    try:
        with sync_playwright() as playwright:
            browser=playwright.chromium.launch(channel="msedge",headless=True)
            desktop=browser.new_page(viewport={"width":1280,"height":800})
            desktop.add_init_script(f"localStorage.setItem('perch_token', {json.dumps(TOKEN)});")
            desktop.goto(f"http://127.0.0.1:{server.server_port}/projects",wait_until="networkidle")
            expect(desktop.locator("#project-current-name")).to_have_text(PROJECT_NAME)
            expect(desktop.locator("#project-job-count")).to_have_text("1")
            expect(desktop.locator("#project-file-count")).to_have_text("3")
            expect(desktop.locator("#project-preflight-state")).to_contain_text("已就绪")
            desktop.locator("#project-edit").click()
            expect(desktop.locator("#project-output-dirs")).to_have_value("reports")
            expect(desktop.locator("#project-preflight-inputs")).to_be_checked()
            desktop.locator("#project-cancel").click()
            desktop.get_by_role("button",name="执行记录").click()
            expect(desktop.locator("#project-runs")).to_be_visible()
            expect(desktop.locator("#project-run-changes")).to_contain_text("generated.xlsx")
            expect(desktop.locator("#project-run-changes")).to_contain_text("自动发现")
            expect(desktop.locator("#project-run-changes")).to_contain_text("新增")
            expect(desktop.locator("#project-run-summary")).to_contain_text("新增 1")
            expect(desktop.locator("#project-run-summary")).to_contain_text("输入预检：通过")
            assert not LOG_READS, "logs must not be fetched automatically"
            desktop.get_by_role("button", name="按需查看该次日志").click()
            expect(desktop.locator("#project-run-log")).to_contain_text("MOCK_LOG_ONLY")
            assert len(LOG_READS) == 1
            desktop.locator("#project-add-python").click()
            expect(desktop.locator("#project-python-modal")).to_be_visible()
            desktop.locator("#project-python-name").fill("mock_python")
            desktop.locator("#project-python-exe").fill(r"C:\MOCK\python.exe")
            desktop.locator("#project-python-script").fill("mock_worker.py")
            desktop.locator("#project-python-args").fill("--limit\n10")
            desktop.locator("#project-python-form").locator('button[type="submit"]').click()
            expect(desktop.locator("#project-python-modal")).to_be_hidden()
            expect(desktop.locator("#project-job-count")).to_have_text("2")
            expect(desktop.locator("#project-file-count")).to_have_text("4")
            assert len(CAPTURED) == 1
            desktop.screenshot(path=str(target/"project-hub-desktop.png"),full_page=True)
            mobile=browser.new_page(viewport={"width":390,"height":844})
            mobile.add_init_script(f"localStorage.setItem('perch_token', {json.dumps(TOKEN)});")
            mobile.goto(f"http://127.0.0.1:{server.server_port}/projects",wait_until="networkidle")
            expect(mobile.locator("#project-current-name")).to_have_text(PROJECT_NAME)
            expect(mobile.locator("#project-add-python")).to_be_visible()
            mobile.screenshot(path=str(target/"project-hub-mobile.png"),full_page=True)
            browser.close()
        (target/"project-hub-browser.json").write_text(json.dumps({
            "desktop_ui":"PASS",
            "desktop_python_creation":"PASS (mock API, task disabled)",
            "run_scoped_file_changes":"PASS (mock API)",
            "opt_in_preflight_display":"PASS (mock API)",
            "run_summary_and_explicit_log":"PASS (mock API)",
            "mobile_ui":"PASS",
            "production_invoked":False
        },indent=2),encoding="utf-8")
        print("PASS project hub isolated desktop/mobile browser test")
    finally:
        server.shutdown()
        server.server_close()

if __name__=="__main__":
    main()
