#!/usr/bin/env python3
"""Standalone local-only PerchFlow frontend preview. Python stdlib only.

Default mock mode never launches commands or alters the real Perch configuration.
Edit frontend/*.html or frontend/assets/*, then refresh the browser. No Rust build.
"""

from __future__ import annotations

import argparse
import copy
import json
import mimetypes
import re
import threading
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import unquote, urlsplit
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
FRONTEND = ROOT / "frontend"
PREVIEW_MARKER = "perchflow-dev-preview"
HTML_ROUTES = {
    "/": "login.html",
    "/processes": "processes.html",
    "/process-detail": "process-detail.html",
    "/tasks": "tasks.html",
}
MIMES = {".css": "text/css", ".js": "text/javascript", ".svg": "image/svg+xml",
         ".woff2": "font/woff2", ".webmanifest": "application/manifest+json"}
DEV_BADGE = (
    '<div aria-label="PerchFlow preview only" '
    'style="position:fixed;bottom:12px;left:12px;z-index:999999;'
    'background:#162d35;color:#a5f1ff;border:1px solid #389bb3;'
    'border-radius:8px;padding:6px 10px;font:600 11px/1.4 system-ui;'
    'letter-spacing:.05em;pointer-events:none;box-shadow:0 6px 18px #0007">'
    'DEV PREVIEW &middot; {label}</div>'
)


def make_process(name: str, status: str, pid: int | None,
                 cpu: float, memory_mb: int, command: str) -> dict:
    return {
        "name": name, "status": status, "pid": pid,
        "uptime_secs": 7311 if status == "running" else 0,
        "restart_count": 1 if status == "crashed" else 0,
        "cpu_percent": cpu, "mem_bytes": memory_mb * 1024 * 1024,
        "cmd": command, "cmd_raw": command.split(" ")[0],
        "args": command.split(" ")[1:], "cwd": None, "autostart": False,
    }


class MockState:
    def __init__(self) -> None:
        self.lock = threading.RLock()
        self.lang = "zh"
        self.processes = {
            p["name"]: p
            for p in [
                make_process("MailHub (DEMO)", "running", 12340, 1.6, 68, "python launcher.py"),
                make_process("Daily_Check (DEMO)", "stopped", None, 0.0, 0, "python daily_check.py"),
                make_process("Video Publisher (DEMO)", "crashed", None, 0.0, 0, "python video_auto.py"),
                make_process("Data Collector (DEMO)", "running", 25678, 3.4, 128, "python collector.py"),
            ]
        }
        self.tasks = {
            t["name"]: t for t in [
                {"name": "Morning Check (DEMO)", "schedule": "0 0 9 * * *",
                 "cmd": "python daily_check.py", "args": [], "cwd": None,
                 "enabled": True, "last_run": "2026-10-19T09:00:00Z", "last_exit_code": 0},
                {"name": "Weekly Report (DEMO)", "schedule": "0 0 10 * * MON",
                 "cmd": "python weekly_report.py", "args": [], "cwd": None,
                 "enabled": False, "last_run": None, "last_exit_code": None},
            ]
        }
        self.logs = {
            "MailHub (DEMO)": [
                {"seq": 0, "text": "[INFO] MailHub preview started (simulated)"},
                {"seq": 1, "text": "[INFO] Listening on 127.0.0.1:9877 (example only)"},
                {"seq": 2, "text": "[INFO] Process tree memory: 68 MB (example only)"},
            ]
        }
        self.next_pid = 32000


def build_handler(frontend: Path = FRONTEND, mode: str = "mock",
                  backend: str | None = None, state: MockState | None = None):
    # Pin the root to this project so absolute traversal cannot expose local files.
    frontend = frontend.resolve(strict=True)
    if not frontend.is_dir():
        raise ValueError("frontend directory is missing")
    state = state or MockState()

    class PreviewHandler(BaseHTTPRequestHandler):
        server_version = "PerchFlowPreview/1.0"
        protocol_version = "HTTP/1.1"

        def log_message(self, format, *args):
            # Never log request URLs, tokens, or Authorization headers.
            return

        def send_bytes(self, status: int, body: bytes, content_type: str,
                       extra: dict | None = None):
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store, max-age=0")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("X-PerchFlow-Preview", PREVIEW_MARKER)
            self.send_header("Referrer-Policy", "no-referrer")
            self.send_header("Cross-Origin-Resource-Policy", "same-origin")
            for key, value in (extra or {}).items():
                self.send_header(key, value)
            self.end_headers()
            if self.command != "HEAD":
                self.wfile.write(body)

        def send_json(self, value, status=200):
            self.send_bytes(status, json.dumps(value, ensure_ascii=False).encode("utf-8"),
                            "application/json; charset=utf-8")

        def json_body(self):
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if length < 1 or length > 65536:
                    return None
                value = json.loads(self.rfile.read(length))
                return value if isinstance(value, dict) else None
            except (ValueError, UnicodeError):
                return None

        def do_HEAD(self):
            self.do_GET()

        def do_GET(self):
            parts = urlsplit(self.path)
            path = unquote(parts.path)
            if path == "/_dev/status":
                return self.send_json({
                    "app": PREVIEW_MARKER, "mode": mode,
                    "frontend": str(frontend), "read_only": mode == "live-readonly",
                    "sample_data": mode == "mock",
                })
            if path.startswith("/api/"):
                return self.api_request("GET", path, parts.query)
            if path in HTML_ROUTES:
                file = frontend / HTML_ROUTES[path]
                if not file.is_file():
                    return self.send_json({"error": "Page not found"}, 404)
                html = file.read_text(encoding="utf-8").replace("{{VERSION}}", "0.1.2-dev")
                badge = DEV_BADGE.format(label="MOCK DATA" if mode == "mock" else "LIVE READ-ONLY")
                html = re.sub(r"(<body\b[^>]*>)", lambda m: m.group(1) + badge,
                              html, count=1, flags=re.IGNORECASE)
                return self.send_bytes(200, html.encode("utf-8"), "text/html; charset=utf-8")
            if path.startswith("/assets/"):
                # Decode once, then canonicalize and refuse traversal/symlinks.
                relative = path.lstrip("/")
                target = (frontend / relative).resolve()
                if not target.is_relative_to(frontend) or not target.is_file():
                    return self.send_json({"error": "Resource not found"}, 404)
                mime = MIMES.get(target.suffix.lower()) or mimetypes.guess_type(target.name)[0] or "application/octet-stream"
                if mime.startswith("text/") or mime in ("application/manifest+json",):
                    mime += "; charset=utf-8"
                return self.send_bytes(200, target.read_bytes(), mime)
            self.send_json({"error": "Not found"}, 404)

        def do_POST(self):
            self.mutation("POST")

        def do_PUT(self):
            self.mutation("PUT")

        def do_DELETE(self):
            self.mutation("DELETE")

        def mutation(self, method: str):
            path = unquote(urlsplit(self.path).path)
            if path == "/_dev/stop":
                # Only direct local tooling can issue this custom-header request;
                # a cross-origin browser form cannot set the required header.
                if self.client_address[0] not in ("127.0.0.1", "::1") or self.headers.get("X-PerchFlow-Control") != "stop-local-preview":
                    return self.send_json({"error": "Forbidden"}, 403)
                self.send_json({"ok": True, "stopping": True})
                threading.Thread(target=self.server.shutdown, daemon=True).start()
                return
            if path.startswith("/api/"):
                if mode == "live-readonly":
                    return self.send_json({"ok": False, "error": "Live preview is read-only; action blocked"}, 403)
                return self.api_request(method, path, "")
            self.send_json({"error": "Not found"}, 404)

        def api_request(self, method: str, path: str, query: str):
            if mode == "live-readonly":
                return self.live_readonly_get(method, path, query)
            return self.mock_api(method, path, query)

        def live_readonly_get(self, method: str, path: str, query: str):
            if method != "GET":
                return self.send_json({"error": "Read-only preview"}, 403)
            if path.startswith("/api/logs/stream/"):
                return self.send_bytes(200,
                    b'data: {"ended":true,"status":"stopped"}\n\n',
                    "text/event-stream; charset=utf-8")
            if not (path == "/api/lang" or
                    path.startswith("/api/processes") or path.startswith("/api/tasks")):
                return self.send_json({"error": "Not found"}, 404)
            target = backend + path + ("?" + query if query else "")
            headers = {"Accept": "application/json"}
            if self.headers.get("Authorization"):
                headers["Authorization"] = self.headers["Authorization"]
            try:
                req = Request(target, headers=headers, method="GET")
                with urlopen(req, timeout=8) as res:
                    data = res.read(4 * 1024 * 1024 + 1)
                    if len(data) > 4 * 1024 * 1024:
                        return self.send_json({"error": "Upstream response too large"}, 502)
                    self.send_bytes(res.status, data,
                        res.headers.get("Content-Type", "application/json"))
            except HTTPError as exc:
                self.send_bytes(exc.code, exc.read(1024 * 1024),
                    exc.headers.get("Content-Type", "application/json"))
            except (URLError, TimeoutError, OSError):
                self.send_json({"error": "Backend unavailable; production was not modified"}, 502)

        def mock_api(self, method: str, path: str, query: str):
            chunks = path.strip("/").split("/")
            with state.lock:
                if path == "/api/lang":
                    if method == "GET":
                        return self.send_json({"lang": state.lang})
                    payload = self.json_body()
                    if method == "PUT" and payload and payload.get("lang") in ("zh", "en", "es", "ja", "pt"):
                        state.lang = payload["lang"]
                        return self.send_json({"ok": True})
                    return self.send_json({"ok": False, "error": "invalid lang"}, 400)

                if path == "/api/processes":
                    if method == "GET":
                        return self.send_json({"processes": copy.deepcopy(list(state.processes.values()))})
                    if method == "POST":
                        obj = self.json_body()
                        if not obj or not obj.get("name") or not obj.get("cmd"):
                            return self.send_json({"ok": False, "error": "name/cmd required"}, 400)
                        if obj["name"] in state.processes:
                            return self.send_json({"ok": False, "error": "name exists"})
                        state.processes[obj["name"]] = mock_view(obj)
                        return self.send_json({"ok": True})

                if path == "/api/tasks":
                    if method == "GET":
                        return self.send_json({"tasks": copy.deepcopy(list(state.tasks.values()))})
                    if method == "POST":
                        obj = self.json_body()
                        if not obj or not obj.get("name") or not obj.get("schedule") or not obj.get("cmd"):
                            return self.send_json({"ok": False, "error": "missing task fields"}, 400)
                        if obj["name"] in state.tasks:
                            return self.send_json({"ok": False, "error": "name exists"})
                        state.tasks[obj["name"]] = mock_task(obj)
                        return self.send_json({"ok": True})

                if len(chunks) >= 3 and chunks[:2] == ["api", "processes"]:
                    name = chunks[2]
                    obj = state.processes.get(name)
                    if obj is None:
                        return self.send_json({"ok": False, "error": "not found"}, 404)
                    if len(chunks) == 3:
                        if method == "GET":
                            return self.send_json(copy.deepcopy(obj))
                        if method == "DELETE":
                            del state.processes[name]
                            state.logs.pop(name, None)
                            return self.send_json({"ok": True})
                        if method == "PUT":
                            payload = self.json_body()
                            if not payload or not payload.get("cmd"):
                                return self.send_json({"ok": False, "error": "cmd required"}, 400)
                            state.processes[name].update(mock_view({**payload, "name": name}))
                            return self.send_json({"ok": True})
                    if len(chunks) == 4 and chunks[3] == "logs" and method == "GET":
                        lines = state.logs.get(name, [])
                        return self.send_json({"name": name, "lines": lines, "seq": len(lines)})
                    if len(chunks) == 4 and chunks[3] in ("start", "stop", "restart") and method == "POST":
                        action = chunks[3]
                        if action == "stop":
                            obj.update(status="stopped", pid=None, uptime_secs=0,
                                       cpu_percent=0.0, mem_bytes=0)
                        else:
                            state.next_pid += 1
                            obj.update(status="running", pid=state.next_pid,
                                       uptime_secs=0, cpu_percent=1.1,
                                       mem_bytes=42 * 1024 * 1024)
                            if action == "restart":
                                obj["restart_count"] += 1
                        return self.send_json({"ok": True})

                if len(chunks) >= 3 and chunks[:2] == ["api", "tasks"]:
                    name = chunks[2]
                    obj = state.tasks.get(name)
                    if obj is None:
                        return self.send_json({"ok": False, "error": "not found"}, 404)
                    if len(chunks) == 3:
                        if method == "GET":
                            return self.send_json(copy.deepcopy(obj))
                        if method == "DELETE":
                            del state.tasks[name]
                            return self.send_json({"ok": True})
                        if method == "PUT":
                            payload = self.json_body()
                            if not payload or not payload.get("name"):
                                return self.send_json({"ok": False, "error": "name required"}, 400)
                            new_name = payload["name"]
                            if new_name != name and new_name in state.tasks:
                                return self.send_json({"ok": False, "error": "name exists"})
                            del state.tasks[name]
                            state.tasks[new_name] = {**mock_task(payload),
                                "last_run": obj["last_run"],
                                "last_exit_code": obj["last_exit_code"]}
                            return self.send_json({"ok": True})
                    if len(chunks) == 4 and chunks[3] == "run" and method == "POST":
                        obj["last_run"] = datetime.now(timezone.utc).isoformat()
                        obj["last_exit_code"] = 0
                        return self.send_json({"ok": True})

                if path.startswith("/api/logs/stream/") and method == "GET":
                    name = chunks[-1]
                    obj = state.processes.get(name)
                    status = obj["status"] if obj else "stopped"
                    if status in ("running", "restarting"):
                        lines = state.logs.get(name, [])
                        data = {"lines": lines, "seq": len(lines)}
                    else:
                        data = {"ended": True, "status": status}
                    text = "data: " + json.dumps(data) + "\n\n"
                    return self.send_bytes(200, text.encode("utf-8"),
                                           "text/event-stream; charset=utf-8")

            self.send_json({"ok": False, "error": "Unknown preview API route"}, 404)

    return PreviewHandler


def mock_view(obj: dict) -> dict:
    name = str(obj["name"])
    command = str(obj["cmd"])
    args = [str(x) for x in obj.get("args", [])]
    return {
        "name": name, "status": "stopped", "pid": None, "uptime_secs": 0,
        "restart_count": 0, "cpu_percent": 0.0, "mem_bytes": 0,
        "cmd": " ".join([command, *args]), "cmd_raw": command,
        "args": args, "cwd": obj.get("cwd"), "autostart": bool(obj.get("autostart")),
    }


def mock_task(obj: dict) -> dict:
    return {
        "name": str(obj["name"]), "schedule": str(obj["schedule"]),
        "cmd": str(obj["cmd"]), "args": obj.get("args", []),
        "cwd": obj.get("cwd"), "enabled": bool(obj.get("enabled")),
        "last_run": None, "last_exit_code": None,
    }


def parse_arguments(argv=None):
    parser = argparse.ArgumentParser(description="PerchFlow local frontend preview (no Rust build).")
    parser.add_argument("--port", type=int, default=9011)
    parser.add_argument("--mode", choices=("mock", "live-readonly"), default="mock")
    parser.add_argument("--backend", default="http://127.0.0.1:9002",
                        help="Existing Perch server for live-readonly mode, loopback only.")
    args = parser.parse_args(argv)
    if not 1 <= args.port <= 65535:
        parser.error("Port must be between 1 and 65535")
    endpoint = urlsplit(args.backend)
    if (endpoint.scheme != "http" or endpoint.hostname not in ("127.0.0.1", "localhost") or
            endpoint.username or endpoint.password or endpoint.path not in ("", "/") or
            endpoint.query or endpoint.fragment or endpoint.port is None):
        parser.error("--backend must be a loopback http://127.0.0.1:PORT URL")
    if args.mode == "live-readonly" and endpoint.port == args.port:
        parser.error("preview and real backend ports cannot match")
    args.backend = "http://127.0.0.1:" + str(endpoint.port)
    return args


def serve(args):
    server = ThreadingHTTPServer(
        ("127.0.0.1", args.port),
        build_handler(mode=args.mode, backend=args.backend))
    server.daemon_threads = True
    # pythonw.exe has no console streams; never fail just because stdout is absent.
    import sys
    if sys.stdout is not None:
        print("PerchFlow preview: http://127.0.0.1:" + str(args.port) + "/processes", flush=True)
        print("Mode: " + args.mode + " | frontend/ files reload on each request", flush=True)
        print("Production EXE and perch.toml remain untouched.", flush=True)
    try:
        server.serve_forever(poll_interval=0.2)
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    serve(parse_arguments())
