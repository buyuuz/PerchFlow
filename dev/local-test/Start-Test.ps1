param([switch]$NoBrowser)

$ErrorActionPreference = 'Stop'
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Exe = Join-Path $Here 'perch.exe'
$Config = Join-Path $Here 'perch.toml'
$State = Join-Path $Here 'test-instance.json'
$Demo = Join-Path $Here 'demo'

function Quote-Toml([string]$Value) {
    return ConvertTo-Json -InputObject $Value -Compress
}

function Get-TestProcess([int]$ProcessId) {
    if ($ProcessId -le 0) { return $null }
    try {
        $Process = Get-Process -Id $ProcessId -ErrorAction Stop
        if ($Process.Path -and
            [string]::Equals(
                [IO.Path]::GetFullPath($Process.Path),
                [IO.Path]::GetFullPath($Exe),
                [StringComparison]::OrdinalIgnoreCase
            )) {
            return $Process
        }
    } catch {}
    return $null
}

function Test-LocalPort([int]$Port) {
    $Client = New-Object System.Net.Sockets.TcpClient
    try {
        $Client.Connect([Net.IPAddress]::Loopback, $Port)
        return $true
    } catch {
        return $false
    } finally {
        $Client.Dispose()
    }
}

function Find-Python {
    $Commands = @(
        Get-Command -Name python.exe -CommandType Application -ErrorAction SilentlyContinue
    )
    foreach ($Command in $Commands) {
        if ($null -eq $Command) { continue }
        try {
            $Out = & $Command.Source -c 'import sys; print(sys.executable)' 2>$null
            if ($LASTEXITCODE -eq 0 -and $Out) {
                $Candidate = [string]($Out | Select-Object -First 1)
                if (Test-Path -LiteralPath $Candidate -PathType Leaf) {
                    return [IO.Path]::GetFullPath($Candidate)
                }
            }
        } catch {}
    }
    try {
        $PyLauncher = Get-Command -Name py.exe -CommandType Application -ErrorAction Stop
        $Out = & $PyLauncher.Source -3 -c 'import sys; print(sys.executable)' 2>$null
        if ($LASTEXITCODE -eq 0 -and $Out) {
            $Candidate = [string]($Out | Select-Object -First 1)
            if (Test-Path -LiteralPath $Candidate -PathType Leaf) {
                return [IO.Path]::GetFullPath($Candidate)
            }
        }
    } catch {}
    return $null
}

if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
    throw 'perch.exe was not found. Extract the entire release ZIP first.'
}
if (-not (Test-Path -LiteralPath (Join-Path $Demo 'mock_worker.py') -PathType Leaf)) {
    throw 'The demo folder is missing. Extract the entire release ZIP first.'
}

if (Test-Path -LiteralPath $State) {
    try {
        $Previous = Get-Content -LiteralPath $State -Raw -Encoding UTF8 | ConvertFrom-Json
        $AlreadyRunning = Get-TestProcess -ProcessId ([int]$Previous.process_id)
        if ($null -ne $AlreadyRunning) {
            Write-Host 'This test instance is already running. Use Stop-Test.cmd before restarting.' -ForegroundColor Yellow
            Write-Host ('Test server PID: ' + $AlreadyRunning.Id)
            exit 0
        }
    } catch {}
    Remove-Item -LiteralPath $State -Force
}

if (-not (Test-Path -LiteralPath $Config)) {
    # A fresh loopback-only port avoids the existing production PerchFlow port.
    $Listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    $Listener.Start()
    try {
        $Port = [int]$Listener.LocalEndpoint.Port
    } finally {
        $Listener.Stop()
    }
    $Bytes = New-Object 'System.Byte[]' 32
    $Random = [Security.Cryptography.RandomNumberGenerator]::Create()
    try { $Random.GetBytes($Bytes) } finally { $Random.Dispose() }
    $Token = [Convert]::ToBase64String($Bytes).TrimEnd('=').Replace('+', '-').Replace('/', '_')

    $ProjectRoot = [IO.Path]::GetFullPath($Demo)
    $Worker = Join-Path $ProjectRoot 'mock_worker.py'
    $Python = Find-Python
    $Lines = New-Object System.Collections.Generic.List[string]
    $Lines.Add('[server]')
    $Lines.Add('bind = ' + (Quote-Toml ("127.0.0.1:" + $Port)))
    $Lines.Add('token = ' + (Quote-Toml $Token))
    $Lines.Add('lang = "zh"')
    $Lines.Add('')
    if ($Python) {
        $Lines.Add('[[task]]')
        $Lines.Add('name = "rc_mock_python"')
        $Lines.Add('cmd = ' + (Quote-Toml $Python))
        $Lines.Add('args = [' + (Quote-Toml $Worker) + ']')
        $Lines.Add('cwd = ' + (Quote-Toml $ProjectRoot))
        $Lines.Add('schedule = "0 0 0 * * *"')
        $Lines.Add('timeout_secs = 60')
        $Lines.Add('enabled = false')
        $Lines.Add('')
    }
    $Lines.Add('[[project]]')
    $Lines.Add('name = "RC Sandbox Demo"')
    $Lines.Add('root = ' + (Quote-Toml $ProjectRoot))
    if ($Python) {
        $Lines.Add('tasks = ["rc_mock_python"]')
    } else {
        $Lines.Add('tasks = []')
    }
    $Lines.Add('inputs = ["mock_worker.py", "input.txt"]')
    $Lines.Add('outputs = ["results.txt"]')
    $Lines.Add('output_dirs = ["output"]')
    $Lines.Add('preflight_inputs = true')
    $Content = $Lines -join [Environment]::NewLine
    # No user-owned PerchFlow config or Windows Task Scheduler item is changed.
    [IO.File]::WriteAllText($Config, $Content + [Environment]::NewLine, (New-Object System.Text.UTF8Encoding($false)))
    Write-Host 'Created a NEW test-only configuration. All scheduled tasks are disabled.' -ForegroundColor Green
    if (-not $Python) {
        Write-Host 'Python was not found. The demo project is available, but no task was created.' -ForegroundColor Yellow
        Write-Host 'Install or configure Python, then add the demo task manually in /projects.'
    }
}

# Refuse non-local bindings and occupied ports instead of touching other services.
$Text = Get-Content -LiteralPath $Config -Raw -Encoding UTF8
$Bind = [regex]::Match($Text, '(?m)^\s*bind\s*=\s*"127\.0\.0\.1:(\d+)"\s*$')
$MatchToken = [regex]::Match($Text, '(?m)^\s*token\s*=\s*"([^"]+)"\s*$')
if (-not $Bind.Success -or -not $MatchToken.Success) {
    throw 'Test config must contain a loopback bind and token. Refusing to start with an unknown configuration.'
}
$Port = [int]$Bind.Groups[1].Value
$Token = $MatchToken.Groups[1].Value
if (Test-LocalPort $Port) {
    throw ('Port ' + $Port + ' is already in use. This launcher will not stop other services.')
}
$Process = Start-Process -FilePath $Exe -ArgumentList 'run' -WorkingDirectory $Here -WindowStyle Hidden -PassThru
$ProcessInfo = @{
    process_id = $Process.Id
    executable = $Exe
    port = $Port
    created_utc = [DateTime]::UtcNow.ToString('o')
}
$ProcessInfo | ConvertTo-Json | Set-Content -LiteralPath $State -Encoding UTF8

$Started = $false
for ($Attempt = 0; $Attempt -lt 80; $Attempt++) {
    Start-Sleep -Milliseconds 250
    $Process.Refresh()
    if ($Process.HasExited) { break }
    if (Test-LocalPort $Port) { $Started = $true; break }
}
if (-not $Started) {
    if ($Process.HasExited) {
        Remove-Item -LiteralPath $State -Force -ErrorAction SilentlyContinue
    }
    throw 'The local test server did not start. See logs/ next to perch.exe.'
}
$Url = 'http://127.0.0.1:' + $Port
Write-Host ''
Write-Host 'PerchFlow RC LOCAL TEST is ready' -ForegroundColor Green
Write-Host ('URL: ' + $Url)
Write-Host ('Login token: ' + $Token)
Write-Host 'Use the Projects menu after logging in.'
Write-Host 'The mock Python task is DISABLED. Enable it only if you want to test its timer.'
Write-Host 'For a safe first run, click Run once on the demo task.'
Write-Host 'To stop this test instance, run Stop-Test.cmd (not the production PerchFlow).'
if (-not $NoBrowser) {
    try { Start-Process $Url | Out-Null } catch {
        Write-Host 'Browser did not open automatically. Copy the URL above.' -ForegroundColor Yellow
    }
}
