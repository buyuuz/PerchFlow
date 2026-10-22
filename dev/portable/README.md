# PerchFlow Portable for Windows x64

This is the real main-branch release build, not the ProjectHub test/demo package.
The distributed executable is **PerchFlow.exe**. The internal Rust/Cargo build target remains `perch.exe`; do not rename `perch.toml`.

## First start
1. Extract the entire ZIP into a NEW directory, e.g. D:\Tools\PerchFlow.
2. Double-click PerchFlow-Start.vbs (it will launch the app and open the browser).
3. First launch creates perch.toml with a cryptographically generated random token.
4. Find your token in perch.toml under [server], and paste it when prompted by the web UI.
5. Stop using PerchFlow-Stop.vbs, which refuses to stop if managed programs are running.

Default URL: http://127.0.0.1:9000/processes
If port 9000 is in use, the first-run initializer selects a free local port from 13000 to 14100 and updates perch.toml.

## Important
- This ZIP includes no credentials, real projects, Python demo or previous configurations.
- Keep perch.toml private. Never distribute it.
- Do NOT overwrite a running PerchFlow.exe or copy its live configuration blindly.
- The stop helper checks for active managed processes; stop these in the UI first.
- The EXE embeds a blue P icon and contains the UI, Rust runtime and Windows integrations.
- Python, dependencies and external projects must be installed separately if managed by PerchFlow.
- Requires Windows x64. No separate Rust/Node runtime is needed.

Source: https://github.com/buyuuz/PerchFlow
Build metadata: BUILD_INFO.txt inside this ZIP.