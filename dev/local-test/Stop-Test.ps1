$ErrorActionPreference = 'Stop'
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Exe = [IO.Path]::GetFullPath((Join-Path $Here 'perch.exe'))
$State = Join-Path $Here 'test-instance.json'
if (-not (Test-Path -LiteralPath $State -PathType Leaf)) {
    Write-Host 'No test instance record found. Nothing to stop.' -ForegroundColor Yellow
    exit 0
}
try {
    $Saved = Get-Content -LiteralPath $State -Raw -Encoding UTF8 | ConvertFrom-Json
    $ProcessId = [int]$Saved.process_id
    if ($ProcessId -le 0) { throw 'invalid recorded process ID' }
    $Process = Get-Process -Id $ProcessId -ErrorAction Stop
} catch {
    Write-Host 'The recorded test process is no longer running.'
    Remove-Item -LiteralPath $State -Force
    exit 0
}
$ActivePath = $Process.Path
if (-not $ActivePath -or -not [string]::Equals(
    [IO.Path]::GetFullPath($ActivePath), $Exe,
    [StringComparison]::OrdinalIgnoreCase)) {
    throw 'The saved PID does NOT match this test package. Refusing to stop any unrelated process.'
}
# This stops only the background server launched from this folder.
# Please finish any running manual mock tasks before stopping.
Stop-Process -Id $ProcessId -ErrorAction Stop
try { Wait-Process -Id $ProcessId -Timeout 8 -ErrorAction SilentlyContinue } catch {}
Remove-Item -LiteralPath $State -Force
Write-Host 'Stopped only the PerchFlow test instance.' -ForegroundColor Green
Write-Host 'Test configuration, scripts and logs were preserved for inspection.'
