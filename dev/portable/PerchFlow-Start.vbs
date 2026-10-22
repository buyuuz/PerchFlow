Option Explicit

Dim fso, shell, folder, exe, config, endpoint, url, serverPort, valid, i, startOnly, initScript, initCommand, initCode
startOnly = False
Set fso = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")
folder = fso.GetParentFolderName(WScript.ScriptFullName)
exe = fso.BuildPath(folder, "perch.exe")
config = fso.BuildPath(folder, "perch.toml")

' Generate a fresh private token on first launch, BEFORE running perch.exe.
If Not fso.FileExists(config) Then
    initScript = fso.BuildPath(folder, "Portable-Init.ps1")
    If Not fso.FileExists(initScript) Then
        MsgBox "Portable-Init.ps1 is missing. Cannot securely initialize the portable version.", vbCritical, "PerchFlow Portable"
        WScript.Quit 8
    End If
    initCommand = "powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -File " & Chr(34) & initScript & Chr(34)
    initCode = shell.Run(initCommand, 0, True)
    If initCode <> 0 Or Not fso.FileExists(config) Then
        MsgBox "Could not initialize perch.toml. Check write permissions and Portable-Init.ps1.", vbCritical, "PerchFlow Portable"
        WScript.Quit 9
    End If
End If

If Not fso.FileExists(exe) Or Not fso.FileExists(config) Then
    MsgBox "未找到 perch.exe 或 perch.toml：" & vbCrLf & folder, vbCritical, "PerchFlow"
    WScript.Quit 1
End If

serverPort = ConfigPort(config)
If serverPort = "" Then
    MsgBox "无法从 perch.toml 读取服务端口。", vbExclamation, "PerchFlow"
    WScript.Quit 2
End If
url = "http://127.0.0.1:" & serverPort & "/processes"
endpoint = "http://127.0.0.1:" & serverPort & "/"

If WScript.Arguments.Count > 0 Then
    Select Case LCase(WScript.Arguments(0))
        Case "/check"
            WScript.Echo "Config URL: " & url
            WScript.Echo "Web UI online: " & CStr(IsPerchReady(endpoint))
            WScript.Quit 0
        Case "/startonly"
            startOnly = True
    End Select
End If

If Not IsPerchReady(endpoint) Then
    If HasPerchServer(fso.GetAbsolutePathName(exe)) Then
        MsgBox "检测到 PerchFlow 后台进程，但当前配置的端口 " & serverPort & " 未就绪。" & vbCrLf & "请先运行 PerchFlow-Stop.vbs，随后重试。", vbExclamation, "PerchFlow"
        WScript.Quit 3
    End If
    Dim wmi, startup, pid, result
    Set wmi = GetObject("winmgmts:\\.\root\cimv2")
    Set startup = wmi.Get("Win32_ProcessStartup").SpawnInstance_
    startup.ShowWindow = 0
    result = wmi.Get("Win32_Process").Create(Chr(34) & exe & Chr(34) & " run", folder, startup, pid)
    If result <> 0 Then
        MsgBox "Windows 无法启动 PerchFlow，WMI 错误码：" & CStr(result), vbExclamation, "PerchFlow"
        WScript.Quit 5
    End If
    valid = False
    For i = 1 To 48
        WScript.Sleep 250
        If IsPerchReady(endpoint) Then
            valid = True
            Exit For
        End If
    Next
    If Not valid Then
        MsgBox "PerchFlow 未能在端口 " & serverPort & " 启动。" & vbCrLf & "请检查 perch.toml 和 logs。", vbExclamation, "PerchFlow"
        WScript.Quit 4
    End If
End If

' If a different PerchFlow instance owns the port, never open or manage its UI.
If IsPerchReady(endpoint) And Not HasPerchServer(fso.GetAbsolutePathName(exe)) Then
    MsgBox "The configured port is being used by another instance. Edit perch.toml bind to use another port.", vbExclamation, "PerchFlow"
    WScript.Quit 7
End If
If Not startOnly Then shell.Run "explorer.exe " & Chr(34) & url & Chr(34), 1, False
WScript.Quit 0

Function ConfigPort(path)
    Dim stream, data, rex, matches, address, lastColon, p
    ConfigPort = ""
    Set stream = fso.OpenTextFile(path, 1, False)
    data = stream.ReadAll
    stream.Close
    Set rex = CreateObject("VBScript.RegExp")
    rex.Pattern = "^\s*bind\s*=\s*[""']([^""']+)[""']"
    rex.MultiLine = True
    rex.IgnoreCase = True
    If Not rex.Test(data) Then Exit Function
    Set matches = rex.Execute(data)
    address = matches(0).SubMatches(0)
    lastColon = InStrRev(address, ":")
    If lastColon = 0 Then Exit Function
    p = Mid(address, lastColon + 1)
    If Not IsNumeric(p) Then Exit Function
    If CLng(p) < 1 Or CLng(p) > 65535 Then Exit Function
    ConfigPort = CStr(CLng(p))
End Function

Function IsPerchReady(address)
    Dim req, body
    IsPerchReady = False
    On Error Resume Next
    Set req = CreateObject("MSXML2.ServerXMLHTTP.6.0")
    req.setTimeouts 700, 700, 700, 700
    req.open "GET", address, False
    req.send
    If Err.Number = 0 Then
        If req.Status = 200 Then
            body = req.responseText
            IsPerchReady = (InStr(1, body, "Perch", vbTextCompare) > 0)
        End If
    End If
    Err.Clear
    On Error GoTo 0
End Function

Function HasPerchServer(fullPath)
    Dim wmi, processes, proc, line
    HasPerchServer = False
    Set wmi = GetObject("winmgmts:\\.\root\cimv2")
    Set processes = wmi.ExecQuery("SELECT ExecutablePath, CommandLine FROM Win32_Process WHERE Name='perch.exe'")
    For Each proc In processes
        If LCase(CStr(proc.ExecutablePath)) = LCase(fullPath) Then
            line = LCase(Trim(CStr(proc.CommandLine)))
            If Right(line, 4) = " run" Then
                HasPerchServer = True
                Exit Function
            End If
        End If
    Next
End Function
