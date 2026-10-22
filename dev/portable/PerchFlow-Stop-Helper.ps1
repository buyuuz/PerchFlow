param([switch]$CheckOnly)

# Operates only on PerchFlow processes whose executable path matches this folder.
# Never terminate the manager while a managed project is running.
$ErrorActionPreference = "Stop"
$folder = Split-Path -Parent $PSCommandPath
$binary = [System.IO.Path]::GetFullPath((Join-Path $folder "PerchFlow.exe"))
$configPath = Join-Path $folder "perch.toml"

try {
    if (!(Test-Path -LiteralPath $binary) -or !(Test-Path -LiteralPath $configPath)) {
        Write-Output "MISSING_BINARY_OR_CONFIG"
        exit 6
    }

    $all = @(Get-CimInstance Win32_Process)
    $perchProcesses = @($all | Where-Object {
        $_.Name -eq "PerchFlow.exe" -and $_.ExecutablePath -ieq $binary
    })
    $servers = @($perchProcesses | Where-Object {
        $_.CommandLine -match '(?i)\srun\s*$'
    })
    if ($servers.Count -eq 0) {
        Write-Output "NOT_RUNNING"
        exit 3
    }

    $configText = [System.IO.File]::ReadAllText($configPath)
    $match = [regex]::Match($configText, '(?m)^\s*token\s*=\s*["'']([^"'']+)["'']')
    if (!$match.Success) {
        Write-Output "TOKEN_NOT_FOUND"
        exit 6
    }
    $authHeaders = @{ Authorization = "Bearer " + $match.Groups[1].Value }
    $listeners = @(Get-NetTCPConnection -State Listen -ErrorAction Stop)
    foreach ($server in $servers) {
        $serverListeners = @($listeners | Where-Object {
            $_.OwningProcess -eq $server.ProcessId -and $_.LocalPort -ge 1
        })
        if ($serverListeners.Count -eq 0) {
            Write-Output ("SERVER_NOT_LISTENING_PID=" + $server.ProcessId)
            exit 5
        }
        foreach ($listener in $serverListeners) {
            if ($listener.LocalPort -ne 9000 -and $listener.LocalPort -ne 9002) {
                # The server may use any configured port; do not hard-code it.
                # Repeated checks are harmless but only HTTP is expected.
            }
            $apiUrl = "http://127.0.0.1:" + $listener.LocalPort + "/api/processes"
            try {
                $info = Invoke-RestMethod -Uri $apiUrl -Method Get -Headers $authHeaders -TimeoutSec 5
            } catch {
                Write-Output ("API_CHECK_FAILED_PORT=" + $listener.LocalPort)
                exit 5
            }
            $activeProjects = @($info.processes | Where-Object {
                $_.status -in @("running", "restarting")
            })
            if ($activeProjects.Count -gt 0) {
                Write-Output ("ACTIVE_PROJECTS=" + (($activeProjects | ForEach-Object { $_.name }) -join ","))
                exit 2
            }
        }
    }

    # Defensive second check: prevent force-stopping orphaned project children
    # even if a project's reported runtime status is stale.
    foreach ($server in $servers) {
        $descendantIds = @([int]$server.ProcessId)
        for ($i=0; $i -lt 8; $i++) {
            foreach ($p in $all) {
                if (($descendantIds -contains [int]$p.ParentProcessId) -and
                    -not ($descendantIds -contains [int]$p.ProcessId)) {
                    $descendantIds += [int]$p.ProcessId
                }
            }
        }
        $unexpected = @($all | Where-Object {
            ($descendantIds -contains [int]$_.ProcessId) -and
            $_.ProcessId -ne $server.ProcessId -and
            $_.Name -ne "conhost.exe"
        })
        if ($unexpected.Count -gt 0) {
            Write-Output ("PROJECT_CHILD_PROCESS_DETECTED_PID=" + $unexpected[0].ProcessId)
            exit 2
        }
    }

    if ($CheckOnly) {
        Write-Output ("READY_TO_STOP=" + $servers.Count)
        exit 0
    }

    # Recheck status immediately before stopping; the caller can run again if
    # a new project was started during preflight.
    foreach ($server in $servers) {
        $serverListeners = @($listeners | Where-Object { $_.OwningProcess -eq $server.ProcessId })
        foreach ($listener in $serverListeners) {
            $info = Invoke-RestMethod -Uri ("http://127.0.0.1:" + $listener.LocalPort + "/api/processes") -Method Get -Headers $authHeaders -TimeoutSec 5
            if (@($info.processes | Where-Object { $_.status -in @("running", "restarting") }).Count -gt 0) {
                Write-Output "ACTIVE_PROJECTS"
                exit 2
            }
        }
    }

    foreach ($server in $servers) {
        Stop-Process -Id $server.ProcessId -Force -ErrorAction Stop
    }
    # The no-argument menu instance is just the terminal launcher.
    foreach ($menu in ($perchProcesses | Where-Object {
        $_.CommandLine -notmatch '(?i)\srun\s*$'
    })) {
        Stop-Process -Id $menu.ProcessId -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Milliseconds 500
    $remain = @(Get-CimInstance Win32_Process -Filter "Name='PerchFlow.exe'" |
        Where-Object { $_.ExecutablePath -ieq $binary })
    if ($remain.Count -gt 0) {
        Write-Output "PERCH_PROCESS_REMAINING"
        exit 5
    }
    Write-Output "STOPPED"
    exit 0
} catch {
    Write-Output "STOP_ERROR"
    exit 5
}
