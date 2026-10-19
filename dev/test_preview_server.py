"""No external dependencies: python -m unittest discover -s dev -p 'test_*.py' -v."""
import http.client
import json
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from preview_server import FRONTEND, build_handler, parse_arguments


class ServerHarness:
    def __init__(self, handler):
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = f"http://127.0.0.1:{self.server.server_port}"

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)


class PreviewTests(unittest.TestCase):
    def setUp(self):
        self.harness = ServerHarness(build_handler(frontend=FRONTEND))

    def tearDown(self):
        self.harness.close()

    def get(self, path):
        with urlopen(self.harness.url + path, timeout=3) as res:
            return res.status, dict(res.headers), res.read()

    def request_json(self, path, method="GET", obj=None):
        data = json.dumps(obj).encode() if obj is not None else None
        req = Request(self.harness.url + path, data=data, method=method)
        with urlopen(req, timeout=3) as res:
            return res.status, json.load(res)

    def test_every_html_route_is_served_and_badged(self):
        for path in ("/", "/processes", "/tasks", "/process-detail"):
            with self.subTest(path=path):
                code, headers, body = self.get(path)
                self.assertEqual(code, 200)
                self.assertIn(b"<html", body)
                self.assertIn(b"DEV PREVIEW", body)
                self.assertIn(b"MOCK DATA", body)
                self.assertNotIn(b"{{VERSION}}", body)
                self.assertEqual(headers["Cache-Control"], "no-store, max-age=0")
                self.assertEqual(headers["X-PerchFlow-Preview"], "perchflow-dev-preview")

    def test_css_js_fonts_and_favicon_load(self):
        for path in ("/assets/tailwind.css", "/assets/app.js", "/assets/i18n.js",
                     "/assets/fonts/inter-400.woff2", "/assets/favicon.svg"):
            with self.subTest(path=path):
                status, headers, content = self.get(path)
                self.assertEqual(status, 200)
                self.assertGreater(len(content), 20)
                expected_cache = "private, max-age=86400" if path.endswith(".woff2") else "no-store, max-age=0"
                self.assertEqual(headers["Cache-Control"], expected_cache)

    def test_font_caching_is_distinct_from_live_editing(self):
        for path in ("/assets/fonts/material-symbols-outlined.woff2",
                     "/assets/fonts/inter-700.woff2"):
            with self.subTest(path=path):
                status, headers, _ = self.get(path)
                self.assertEqual(status, 200)
                self.assertEqual(headers["Cache-Control"], "private, max-age=86400")
        # Changing frontend HTML and JS must remain visible immediately.
        for path in ("/processes", "/assets/processes.js",
                     "/assets/branding.css", "/assets/process-table.css"):
            _, headers, _ = self.get(path)
            self.assertEqual(headers["Cache-Control"], "no-store, max-age=0")

    def test_mock_data_and_process_mutations_are_in_memory(self):
        _, processes = self.request_json("/api/processes")
        self.assertGreaterEqual(len(processes["processes"]), 3)
        example = next(p for p in processes["processes"] if p["status"] == "stopped")
        self.assertIn("cmd_raw", example)
        self.assertEqual(example["cpu_percent"], 0.0)
        name = example["name"].replace(" ", "%20")
        _, result = self.request_json(f"/api/processes/{name}/start", method="POST")
        self.assertTrue(result["ok"])
        _, new = self.request_json(f"/api/processes/{name}")
        self.assertEqual(new["status"], "running")
        self.assertGreater(new["mem_bytes"], 0)
        _, result = self.request_json(f"/api/processes/{name}/stop", method="POST")
        self.assertTrue(result["ok"])
        _, stopped = self.request_json(f"/api/processes/{name}")
        self.assertEqual(stopped["status"], "stopped")

    def test_add_edit_delete_mock_project(self):
        obj = {"name": "UI Test", "cmd": "python", "args": ["demo.py"], "autostart": False}
        _, res = self.request_json("/api/processes", method="POST", obj=obj)
        self.assertTrue(res["ok"])
        _, result = self.request_json("/api/processes/UI%20Test", method="PUT",
                                      obj={**obj, "cmd": "node", "args": ["demo.js"]})
        self.assertTrue(result["ok"])
        _, view = self.request_json("/api/processes/UI%20Test")
        self.assertEqual(view["cmd"], "node demo.js")
        _, result = self.request_json("/api/processes/UI%20Test", method="DELETE")
        self.assertTrue(result["ok"])
        _, process_list = self.request_json("/api/processes")
        self.assertFalse(any(p["name"] == "UI Test" for p in process_list["processes"]))

    def test_mock_tasks_crud_and_logs(self):
        _, data = self.request_json("/api/tasks")
        self.assertTrue(data["tasks"])
        obj = {"name": "UI Task", "cmd": "python test.py", "schedule": "0 0 * * * *",
               "args": [], "enabled": True}
        _, result = self.request_json("/api/tasks", method="POST", obj=obj)
        self.assertTrue(result["ok"])
        _, result = self.request_json("/api/tasks/UI%20Task/run", method="POST")
        self.assertTrue(result["ok"])
        _, task = self.request_json("/api/tasks/UI%20Task")
        self.assertEqual(task["last_exit_code"], 0)
        _, result = self.request_json("/api/tasks/UI%20Task", method="DELETE")
        self.assertTrue(result["ok"])
        _, logs = self.request_json("/api/processes/MailHub%20(DEMO)/logs?lines=50")
        self.assertGreater(logs["seq"], 0)
        self.assertIsInstance(logs["lines"][0]["text"], str)
        status, headers, sse = self.get("/api/logs/stream/MailHub%20(DEMO)?token=dummy")
        self.assertEqual(status, 200)
        self.assertTrue(sse.startswith(b"data: "))
        self.assertTrue(sse.endswith(b"\n\n"))
        self.assertIn("text/event-stream", headers["Content-Type"])

    def test_prevent_path_traversal(self):
        conn = http.client.HTTPConnection("127.0.0.1", self.harness.server.server_port)
        conn.request("GET", "/assets/%2e%2e/%2e%2e/Cargo.toml")
        res = conn.getresponse()
        self.assertEqual(res.status, 404)
        self.assertNotIn(b"[package]", res.read())
        conn.close()

    def test_file_changes_visible_without_restart(self):
        with tempfile.TemporaryDirectory() as directory:
            frontend = Path(directory)
            (frontend / "assets").mkdir()
            page = frontend / "processes.html"
            page.write_text("<html><body>before</body></html>", encoding="utf-8")
            instance = ServerHarness(build_handler(frontend=frontend))
            try:
                with urlopen(instance.url + "/processes", timeout=3) as response:
                    self.assertIn(b"before", response.read())
                page.write_text("<html><body>after</body></html>", encoding="utf-8")
                with urlopen(instance.url + "/processes", timeout=3) as response:
                    self.assertIn(b"after", response.read())
            finally:
                instance.close()

    def test_mutating_live_preview_is_blocked(self):
        class Backend(BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass
            def do_GET(self):
                b = b'{"processes":[]}'
                self.send_response(200)
                self.send_header("Content-Length", str(len(b)))
                self.end_headers()
                self.wfile.write(b)
        backend = ServerHarness(Backend)
        live = ServerHarness(build_handler(mode="live-readonly", backend=backend.url))
        try:
            with urlopen(live.url + "/api/processes", timeout=3) as result:
                self.assertEqual(json.load(result), {"processes": []})
            with self.assertRaises(HTTPError) as ctx:
                urlopen(Request(live.url + "/api/processes/a/start",
                                data=b"{}", method="POST"), timeout=3)
            self.assertEqual(ctx.exception.code, 403)
        finally:
            live.close()
            backend.close()

    def test_preview_binding_options_are_local(self):
        with self.assertRaises(SystemExit):
            parse_arguments(["--mode", "live-readonly", "--backend",
                             "http://8.8.8.8:9002"])
        with self.assertRaises(SystemExit):
            parse_arguments(["--mode", "live-readonly", "--backend",
                             "http://localhost:9011"])
        self.assertEqual(parse_arguments(["--port", "9013"]).port, 9013)

    def test_remote_stop_requires_custom_header(self):
        with self.assertRaises(HTTPError) as ctx:
            urlopen(Request(self.harness.url + "/_dev/stop", data=b"",
                            method="POST"), timeout=3)
        self.assertEqual(ctx.exception.code, 403)
        status, obj = self.request_json("/_dev/status")
        self.assertEqual(obj["app"], "perchflow-dev-preview")
        self.assertTrue(obj["sample_data"])


if __name__ == "__main__":
    unittest.main()
