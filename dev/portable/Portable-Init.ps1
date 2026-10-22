# PerchFlow Portable first-launch initialization.
# Generates a local-only config with a unique cryptographic token.
# Never overwrites an existing perch.toml.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSCommandPath
$config = Join-Path $root 'perch.toml'
if ([IO.File]::Exists($config)) {
    exit 0
}

$temp = Join-Path $root ('perch.toml.init-' + $PID + '.tmp')
try {
    $bytes = New-Object byte[] 32
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $rng.GetBytes($bytes)
    } finally {
        $rng.Dispose()
    }
    $token = [BitConverter]::ToString($bytes).Replace('-', '').ToLowerInvariant()
    # Use the standard port where possible; an isolated second copy picks a free port.
    $port = 9000
    for ($tryPort=9000; $tryPort -le 14100; $tryPort++) {
        if ($tryPort -gt 9000 -and $tryPort -lt 13000) { continue }
        $probe = New-Object System.Net.Sockets.TcpListener([Net.IPAddress]::Loopback, $tryPort)
        try {
            $probe.Start()
            $probe.Stop()
            $port = $tryPort
            break
        } catch {
            try { $probe.Stop() } catch {}
            if ($tryPort -eq 14100) { throw "No free local port available" }
        }
    }    $content = @"
# PerchFlow Portable - generated on this computer.
# This file is private. Never send it with a shared portable ZIP.
# To change port, edit the bind value while the manager is stopped.
[server]
bind = "127.0.0.1:$port"
token = "$token"
lang = "zh"
"@
    $encoding = New-Object System.Text.UTF8Encoding($false)
    [IO.File]::WriteAllText($temp, ($content.Trim() + "`r`n"), $encoding)

    # File.Move with a non-overwrite destination: avoid replacing existing config.
    if ([IO.File]::Exists($config)) {
        Remove-Item -LiteralPath $temp -Force
        exit 0
    }
    [IO.File]::Move($temp, $config)
    exit 0
} catch {
    if ([IO.File]::Exists($temp)) {
        Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue
    }
    Write-Error ('Unable to initialize PerchFlow Portable: ' + $_.Exception.Message)
    exit 1
}
