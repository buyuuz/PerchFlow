#!/usr/bin/env python3
"""Headless Edge Docker V1 UI regression using mocked HTTP API; no Docker needed."""
from contextlib import contextmanager
from functools import partial
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from threading import Thread
import json
from playwright.sync_api import sync_playwright

FRONTEND=Path(__file__).resolve().parents[1]/"frontend"
A="a"*64;B="b"*64
class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, *args): pass

@contextmanager
def web_server():
    server=ThreadingHTTPServer(("127.0.0.1",0),partial(QuietHandler,directory=str(FRONTEND)))
    thread=Thread(target=server.serve_forever,daemon=True);thread.start()
    try:yield server.server_address[1]
    finally:server.shutdown();server.server_close()

with web_server() as port, sync_playwright() as p:
    browser=p.chromium.launch(channel="msedge",headless=True)
    page=browser.new_page(viewport={"width":1250,"height":800})
    mock={"running":True,"disconnected":False,"fail_restart":False}
    def container(id,name,status,exit_code):
        return {"id":id,"short_id":id[:12],"name":name,"image":"example:latest",
                "status":status,"status_text":"Up 2 hours" if status=="running" else "Exited",
                "cpu_percent":0.2 if status=="running" else None,
                "memory_mib":100.8 if status=="running" else None,
                "started_at":"2026-10-22T06:00:00Z","uptime_seconds":4320 if status=="running" else None,
                "restart_count":2,"exit_code":exit_code}
    def handle(route):
        url=route.request.url
        if mock["disconnected"]:
            route.fulfill(status=200,content_type="application/json",body=json.dumps({"ok":False,"connected":False,"error":"engine_unavailable","message":"Mock Docker pipe denied"}))
            return
        rows=[container(A,"example-worker","running" if mock["running"] else "exited",0),
              container(B,"example-db","exited",1)]
        if url.endswith("/api/docker/containers"):
            body={"ok":True,"connected":True,"containers":rows,"updated_at":"2026-10-22T08:00:00Z"}
        elif url.endswith("/"+A):
            body={"ok":True,"connected":True,"container":rows[0]}
        elif "/logs?" in url:
            body={"ok":True,"logs":"2026-10-22T08:00:00Z Mock STDOUT line\n","truncated":False,"tail":"200"}
        elif url.endswith(("/stop","/start","/restart")):
            if url.endswith("/restart") and mock["fail_restart"]:
                body={"ok":False,"connected":True,"error":"docker_error","message":"Mock restart refused"}
            else:
                mock["running"]=not url.endswith("/stop")
                body={"ok":True,"connected":True,"status":"running" if mock["running"] else "exited"}
        else: body={"ok":False,"error":"not_found","message":url}
        route.fulfill(status=200,content_type="application/json",body=json.dumps(body))
    page.route("**/api/docker/**",handle)
    page.goto(f"http://127.0.0.1:{port}/docker.html",wait_until="domcontentloaded")
    page.locator("#docker-total").get_by_text("2").wait_for(timeout=12000)
    assert page.locator("#docker-running").inner_text()=="1"
    assert page.locator("#docker-stopped").inner_text()=="1"
    page.locator('tr[data-id="'+A+'"]').click()
    page.locator("#docker-name").get_by_text("example-worker").wait_for()
    page.locator("#docker-log-text").get_by_text("Mock STDOUT line",exact=False).wait_for()
    assert page.locator("#docker-start").is_disabled()
    page.locator("#docker-stop").click()
    assert page.locator(".pf-dialog-overlay").count()==1,"Stop should require confirmation"
    page.locator(".pf-dialog-primary").click()
    page.locator("#docker-start").wait_for(state="visible")
    page.wait_for_function("document.querySelector('#docker-start').disabled===false",timeout=12000)
    assert page.locator("#docker-stop").is_disabled()
    page.locator("#docker-start").click()
    page.wait_for_function("document.querySelector('#docker-start').disabled===true",timeout=12000)
    mock["fail_restart"]=True
    page.locator("#docker-restart").click()
    page.locator(".pf-dialog-primary").click()
    page.locator("#docker-error").get_by_text("Mock restart refused").wait_for()
    page.locator("#docker-refresh").click()
    page.locator("#docker-error").get_by_text("Mock restart refused").wait_for()
    mock["disconnected"]=True
    page.locator("#docker-refresh").click()
    page.locator("#docker-error").get_by_text("Mock Docker pipe denied").wait_for()
    assert page.locator("#docker-total").inner_text()=="—","Offline must not show cached state"
    assert "Mock STDOUT" not in page.locator("#docker-log-text").inner_text(),"Offline detail must not show stale logs"
    assert "100.8" not in page.locator("#docker-info").inner_text(),"Offline detail must not show stale resource readings"
    assert page.locator("#docker-start").is_disabled()
    assert page.locator("#docker-stop").is_disabled()
    assert page.locator("#docker-restart").is_disabled()
    page.locator("#docker-back").click()
    assert page.locator('tr[data-id]').count()==0
    browser.close()
    print("DOCKER_UI_V1_OK: nav/list, 2 counts, details, logs, safe stop confirm, state updates, offline clearing")
