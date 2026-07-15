<div align="center">

# 🐦 Perch

**A lightweight, single-binary process & service manager for Windows, with a built-in web UI.**

Written in Rust. No runtime dependencies. One binary, double-click and go.

</div>

---

## Why Perch?

Managing background processes and long-running services on Windows is more painful than it should be:

- **Task Scheduler** kills long-running processes by default (session timeouts, process-tree termination, login-session dependencies) — it was never meant for daemons.
- **NSSM / WinSW / Shawl** are one-wrapper-per-service. No dashboard, no central view, register N times.
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
- **Liveness monitoring** — health checks to detect and recover stuck processes.
- **Notifications** — get alerted when a process dies or a check fails.

## Status

> ⚠️ **Early development.** Perch is under active construction and not yet production-ready. Interfaces and configuration may change.

## Quick Start

_Coming soon._

```
# Register Perch as a Windows service
perch install

# Open the web UI
http://127.0.0.1:9000
```

## Security

The management endpoint can start and stop arbitrary processes — it is **highly privileged**. Perch binds to `127.0.0.1` by default and requires an access token. Do not expose the management port to untrusted networks.

## Comparison

| | Perch | NSSM / WinSW | PM2 | Supervisor |
|---|:---:|:---:|:---:|:---:|
| Single binary | ✅ | ✅ | ❌ (needs Node) | ❌ (needs Python) |
| Manages many processes from one instance | ✅ | ❌ (1 per service) | ✅ | ✅ |
| Built-in web UI | ✅ | ❌ | ✅ | ✅ |
| Native Windows service | ✅ | ✅ | ⚠️ | ❌ |
| Scheduled tasks | ✅ | ❌ | ✅ | ⚠️ |
| Low memory footprint | ✅ | ✅ | ❌ | ⚠️ |

## Building from Source

```
cargo build --release
```

Requires a recent stable Rust toolchain.

## License

TBD

---

<div align="center">
<sub>🇨🇳 <a href="./README.zh-CN.md">中文文档</a></sub>
</div>
