<div align="center">
<sub>🇬🇧 English &nbsp;|&nbsp; 🇨🇳 <a href="./README.zh-CN.md">中文</a></sub>
</div>

<br/>

<div align="center">

<img src="logo.png" alt="Perch" width="120">

# Perch

**A lightweight, single-binary process & service manager for Windows, with a built-in web UI.**

Written in Rust. No runtime dependencies. One binary, double-click and go.

</div>

---

## Screenshots

> Place the screenshot files at the paths below (see [Screenshot files to provide](#screenshot-files-to-provide)).

### Interactive Menu (double-click `perch.exe`)

![menu](docs/screenshots/menu.png)

### Login

![login](docs/screenshots/login.png)

### Processes Dashboard

![processes](docs/screenshots/processes.png)

### Process Detail — Live Log Streaming

![process-detail](docs/screenshots/process-detail.png)

### Scheduled Tasks

![tasks](docs/screenshots/tasks.png)

---

## Why Perch?

Managing background processes and long-running services on Windows is more painful than it should be:

- **Task Scheduler** kills long-running processes by default (session timeouts, process-tree termination, login-session dependencies) — it was never meant for daemons.
- **NSSM / WinSw / Shawl** are one-wrapper-per-service. No dashboard, no central view, register N times.
- **PM2** needs a Node.js runtime, its daemon idles at 40–80 MB, and it's a second-class citizen on Windows (unreliable startup, registry hacks).
- **Supervisor** doesn't run on Windows at all.

Perch fills the gap: **register once, manage everything.** A single native service supervises all your processes, exposes a web UI, and stays out of the way.

## Features

- **Single binary** — no runtime, no dependencies. The web UI is embedded directly in the executable.
- **Native Windows service** — registers once, starts on boot, survives logout, runs in Session 0.
- **Low footprint** — compiled Rust, single supervisor process. Targets 5–15 MB resident.
- **Built-in web UI** — view status, start/stop/restart processes, tail logs from your browser. Manage remotely, not just from the console.
- **Auto-restart** — crashed processes are brought back up automatically, with configurable backoff.
- **Scheduled tasks** — cron-style scheduling for periodic jobs.
- **Daily-rotating logs** — application logs written to `logs/perch.YYYY-MM-DD.log` next to the executable, pipe-delimited format.
- **Interactive menu** — double-click the exe to install/uninstall/start/stop the service and open the Web UI, with on-demand UAC elevation.

## Status

> ⚠️ **Early development.** Perch is under active construction and not yet production-ready. Interfaces and configuration may change.

## Quick Start

Double-click `perch.exe` and select `5) Open Web UI (打开 Web UI)` from the menu — Perch will auto-generate a default `perch.toml` (with a `demo` process and token `change-me`) and launch the server.

Or from the command line:

```
# Run in foreground (Ctrl-C to stop)
perch run

# Register Perch as a Windows service (needs admin)
perch install
perch start

# Open the web UI
http://127.0.0.1:9000
```

The default token is `change-me` — change it in `perch.toml` before any real use.

## Security

The management endpoint can start and stop arbitrary processes — it is **highly privileged**. Perch binds to `127.0.0.1` by default and requires an access token. Do not expose the management port to untrusted networks.

## Comparison

| | Perch | NSSM / WinSw | PM2 | Supervisor |
|---|:---:|:---:|:---:|:---:|
| Single binary | ✅ | ✅ | ❌ (needs Node) | ❌ (needs Python) |
| Manages many processes from one instance | ✅ | ❌ (1 per service) | ✅ | ✅ |
| Built-in web UI | ✅ | ❌ | ✅ | ✅ |
| Native Windows service | ✅ | ✅ | ⚠️ | ❌ |
| Scheduled tasks | ✅ | ❌ | ✅ | ⚠️ |
| Daily-rotating app logs | ✅ | ❌ | ⚠️ | ⚠️ |
| Low memory footprint | ✅ | ✅ | ❌ | ⚠️ |

## Building from Source

```
cargo build --release
```

Requires a recent stable Rust toolchain.

## Screenshot files to provide

Put the screenshot image files under `docs/screenshots/` (create the directory first), using these exact filenames:

| Path | What to capture |
|---|---|
| `docs/screenshots/menu.png` | Console window after double-clicking `perch.exe`, showing the bilingual menu (Install/Uninstall/Start/Stop/Open Web UI/Exit). |
| `docs/screenshots/login.png` | `http://127.0.0.1:9000/` login page with the Perch logo and the token input. |
| `docs/screenshots/processes.png` | `/processes` dashboard with the `demo` card showing Running status, and the top nav bar with the logo. |
| `docs/screenshots/process-detail.png` | `/process-detail?name=demo` page with PID/CPU/Mem/Uptime panel and the live log stream. |
| `docs/screenshots/tasks.png` | `/tasks` page with at least one scheduled task row, top nav bar with the logo. |

Tip: capture at a 1280×800 viewport; use PNG; crop the browser chrome. Once the files are in place, the image references above will render automatically.

## License

TBD
