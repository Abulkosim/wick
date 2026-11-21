#Requires -Version 7
# Runs a built Wick on Windows: it must start, hold a power request that powercfg lists under
# wick.exe while awake, report status, and let go on stop and on a forced kill.
# powercfg /requests needs an elevated shell; GitHub's Windows runners are elevated.
# Also checks wick hold, wick release and wick run; WICK_SMOKE_HOLDS=0 skips them. Then the
# console wick-cli.exe beside wick.exe: cmd waits for it, shows its output and gets its exit code.
param([Parameter(Mandatory)][string]$Bin)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$work = Join-Path ([IO.Path]::GetTempPath()) "wick-smoke-$PID"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$log = Join-Path $work 'wick.log'
$errorLog = Join-Path $work 'wick-error.log'
$script:launched = $null
$script:appPid = $null
$script:runner = $null
$script:status = $null
$script:Cli = $null

# Starts a Wick command with its output captured; Complete-Wick waits for it.
function Start-Wick([string[]]$Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new($Bin)
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($info)
    [pscustomobject]@{
        Process = $process
        Command = "wick $($Arguments -join ' ')"
        Out = $process.StandardOutput.ReadToEndAsync()
        Err = $process.StandardError.ReadToEndAsync()
    }
}

function Complete-Wick($run, [int]$Seconds = 30) {
    if (-not $run.Process.WaitForExit($Seconds * 1000)) {
        $run.Process.Kill($true)
        throw "$($run.Command) did not finish within ${Seconds}s"
    }
    $run.Process.WaitForExit()
    [pscustomobject]@{ Code = $run.Process.ExitCode; Out = $run.Out.Result; Err = $run.Err.Result }
}

function Invoke-Wick([string[]]$Arguments) {
    $result = Complete-Wick (Start-Wick $Arguments)
    if ($result.Code -ne 0) { throw "wick $($Arguments -join ' ') exited with $($result.Code): $($result.Err)" }
    $result
}

function Get-Status {
    $script:status = (Invoke-Wick 'status').Out | ConvertFrom-Json
    $script:status
}

function Assert-Status([string]$Name, $Expected, [string]$What) {
    $property = $script:status.PSObject.Properties[$Name]
    $actual = if ($property) { $property.Value } else { '(missing)' }
    if ($actual -cne $Expected) { throw "${What}: expected $Name $Expected, got $actual" }
}

# The app takes commands asynchronously and writes status.json after it changes the power
# request, so a status read right after a command can still show the previous state.
function Wait-Status([string]$Name, $Expected, [string]$What) {
    $deadline = (Get-Date).AddSeconds(15)
    while ($true) {
        Get-Status | Out-Null
        $property = $script:status.PSObject.Properties[$Name]
        if ($property -and $property.Value -ceq $Expected) { return }
        if ((Get-Date) -gt $deadline) { Assert-Status $Name $Expected "$What (waited 15s)" }
        Start-Sleep -Milliseconds 500
    }
}

function Wait-Mode([string]$Mode, [string]$What) {
    Wait-Until "$What (session mode $($script:status.session.mode))" { (Get-Status).session.mode -ceq $Mode }
}

# Polls a check every half second for at most $Seconds seconds.
function Wait-Until([string]$What, [scriptblock]$Check, [int]$Seconds = 15) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    while (-not (& $Check)) {
        if ((Get-Date) -gt $deadline) { throw "$What (waited ${Seconds}s)" }
        Start-Sleep -Milliseconds 500
    }
}

# The powercfg /requests sections (SYSTEM, DISPLAY, ...) holding a request from wick.exe whose
# reason is one of Wick's: "[PROCESS] \Device\...\wick.exe" followed by "Wick: awake ...".
function Get-WickRequests {
    $lines = @(powercfg /requests)
    if ($LASTEXITCODE -ne 0) { throw "powercfg /requests failed with $LASTEXITCODE`: $($lines -join ' ')" }
    $section = $null
    $found = @()
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = "$($lines[$i])".Trim()
        if ($line -cmatch '^([A-Z]+):$') { $section = $Matches[1]; continue }
        if ($line -match '^\[PROCESS\]\s.*\\wick\.exe$' -and $i + 1 -lt $lines.Count -and "$($lines[$i + 1])".Trim().StartsWith('Wick: ')) {
            $found += $section
        }
    }
    @($found | Sort-Object -Unique)
}

function Test-Released { @(Get-WickRequests).Count -eq 0 }

# keepDisplayOn adds a DISPLAY request to the SYSTEM one.
function Assert-Requests([string]$What, [int]$Seconds = 15) {
    $expected = if ($script:status.keepDisplayOn) { 'DISPLAY SYSTEM' } else { 'SYSTEM' }
    $deadline = (Get-Date).AddSeconds($Seconds)
    while (($got = "$(Get-WickRequests)") -cne $expected) {
        if ((Get-Date) -gt $deadline) {
            throw "${What}: expected $expected requests from wick.exe, got $(if ($got) { $got } else { 'none' }) (waited ${Seconds}s)"
        }
        Start-Sleep -Milliseconds 500
    }
    Write-Host "awake: $got requests from wick.exe"
}

function Test-Holds {
    Invoke-Wick 'stop' | Out-Null
    Wait-Until 'request left before the hold checks' { Test-Released }

    # Until the agents folder exists Wick reads it on the idle tick, once a minute.
    Invoke-Wick 'hold', 'smoke', '--pid', "$PID" | Out-Null
    Assert-Requests 'wick hold' 75
    Wait-Status 'awake' $true 'not awake while a hold works'
    Invoke-Wick 'release', 'smoke' | Out-Null
    Wait-Until 'request left after wick release' { Test-Released }
    Wait-Status 'awake' $false 'still awake after wick release'

    $script:runner = Start-Wick 'run', '--', 'ping', '-n', '21', '127.0.0.1'
    Assert-Requests 'wick run'
    $result = Complete-Wick $script:runner
    $script:runner = $null
    if ($result.Code -ne 0) { throw "wick run exited with $($result.Code) for a command that succeeded: $($result.Err)" }
    Wait-Until "request left after wick run's command ended" { Test-Released }
    Wait-Status 'awake' $false "still awake after wick run's command ended"

    $result = Complete-Wick (Start-Wick 'run', '--', 'cmd', '/c', 'exit 7')
    if ($result.Code -ne 7) { throw "wick run exited with $($result.Code), expected the command's 7" }
    Write-Host 'holds: hold, release and run pass.'
}

# The Subsystem field of a Windows program's PE header: 2 is GUI, 3 is console.
function Get-Subsystem([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($pe -lt 0 -or $pe + 94 -gt $bytes.Length -or [Text.Encoding]::ASCII.GetString($bytes, $pe, 4) -cne "PE`0`0") {
        throw "$Path is not a Windows program"
    }
    [BitConverter]::ToUInt16($bytes, $pe + 24 + 68)
}

# wick.exe must stay GUI (no console window from the Start menu or autostart) and wick-cli.exe
# must be a real console build of Wick, never a placeholder.
function Assert-Programs([string]$App, [string]$Cli) {
    if ((Get-Subsystem $App) -ne 2) { throw "$App is not a GUI-subsystem program" }
    if (-not (Test-Path -LiteralPath $Cli -PathType Leaf)) { throw "no wick-cli.exe beside $App" }
    if ((Get-Subsystem $Cli) -ne 3) { throw "$Cli is not a console program" }
    $size = (Get-Item -LiteralPath $Cli).Length
    if ($size -lt 200KB) { throw "$Cli is only $size bytes" }
}

# Runs a line through cmd /c as someone would type it, without redirecting Wick: Wick shares
# cmd's output, which this script reads. A GUI-subsystem wick neither waits nor prints here.
function Invoke-Cmd([string]$Line, [int]$Seconds = 30) {
    $info = [Diagnostics.ProcessStartInfo]::new("$env:SystemRoot\System32\cmd.exe")
    $info.Arguments = "/d /s /c `"$Line`""
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($info)
    $out = $process.StandardOutput.ReadToEndAsync()
    $err = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($Seconds * 1000)) {
        $process.Kill($true)
        throw "cmd /c $Line did not finish within ${Seconds}s"
    }
    $process.WaitForExit()
    [pscustomobject]@{ Code = $process.ExitCode; Out = $out.Result; Err = $err.Result }
}

# The same in a console of its own, the way a terminal runs it; only the exit code comes back.
function Invoke-Console([string]$Line, [int]$Seconds = 30) {
    $process = Start-Process -FilePath "$env:SystemRoot\System32\cmd.exe" -ArgumentList "/d /s /c `"$Line`"" `
        -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit($Seconds * 1000)) {
        $process.Kill($true)
        throw "cmd /c $Line in a new console did not finish within ${Seconds}s"
    }
    $process.WaitForExit()
    $process.ExitCode
}

function Test-Console {
    $cli = "`"$script:Cli`""
    $code = Invoke-Console "$cli run -- cmd /c exit 7"
    if ($code -ne 7) { throw "wick-cli run in a console exited with $code, expected the command's 7" }
    $result = Invoke-Cmd "$cli run -- cmd /c exit 7"
    if ($result.Code -ne 7) { throw "cmd /c wick-cli run exited with $($result.Code), expected 7" }
    # cmd /c waits for GUI programs too, only an interactive prompt doesn't; output tells them apart.
    $gui = Invoke-Console "`"$script:Bin`" run -- cmd /c exit 7"
    $guiOut = (Invoke-Cmd "`"$script:Bin`" status").Out.Trim()
    Write-Host "console: wick-cli run returns 7 through cmd /c; the GUI wick.exe returns $gui and prints $(if ($guiOut) { $guiOut.Length } else { 'no' }) characters for status."

    $result = Invoke-Cmd "$cli run -- cmd /c echo wick-console-check"
    if ($result.Code -ne 0 -or $result.Out -notmatch 'wick-console-check') {
        throw "wick-cli run printed nothing to cmd's console (exit $($result.Code)): $($result.Out) $($result.Err)"
    }
    $result = Invoke-Cmd "$cli status"
    $value = if ($result.Out.Trim()) { $result.Out | ConvertFrom-Json } else { $null }
    if ($result.Code -ne 0 -or -not $value -or $value.running -ne $true -or $value.pid -ne $script:appPid) {
        throw "wick-cli status printed $($result.Out) (exit $($result.Code))"
    }
    $result = Invoke-Cmd "$cli start 0"
    if ($result.Code -ne 2 -or $result.Err -notmatch 'Expected minutes') {
        throw "wick-cli start 0 exited with $($result.Code): $($result.Err)"
    }

    # Commands for the app go to the wick.exe beside wick-cli.exe.
    $result = Invoke-Cmd "$cli start 1"
    if ($result.Code -ne 0) { throw "wick-cli start 1 exited with $($result.Code): $($result.Err)" }
    Wait-Mode 'for' 'wick-cli start 1 is not a timed session'
    Assert-Requests 'wick-cli start 1'
    $result = Invoke-Cmd "$cli stop"
    if ($result.Code -ne 0) { throw "wick-cli stop exited with $($result.Code): $($result.Err)" }
    Wait-Until 'request left after wick-cli stop' { Test-Released }
    Wait-Mode 'off' 'session still on after wick-cli stop'
    Write-Host 'console: run, status, start and stop pass through cmd.'
}

function Test-Wick {
    $principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'powercfg /requests needs an elevated shell; run this from an administrator PowerShell.'
    }
    if (-not (Test-Path -LiteralPath $Bin -PathType Leaf)) { throw "no Wick binary at $Bin" }
    $script:Bin = (Resolve-Path -LiteralPath $Bin).Path
    $script:Cli = Join-Path (Split-Path -Parent $script:Bin) 'wick-cli.exe'
    Assert-Programs $script:Bin $script:Cli

    $script:launched = Start-Process -FilePath $Bin -ArgumentList '--background' -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError $errorLog
    Wait-Until 'Wick did not start' { (Get-Status).running -eq $true } -Seconds 30
    $script:appPid = [int]$script:status.pid
    if ($script:appPid -ne $script:launched.Id) { throw "status names pid $($script:appPid), but Wick runs as $($script:launched.Id)" }
    Assert-Status 'awake' $false 'awake before any start'
    if (-not (Test-Released)) { throw "a request from wick.exe exists before any start: $(Get-WickRequests)" }

    Invoke-Wick 'start', '1' | Out-Null
    Wait-Mode 'for' 'start 1 is not a timed session'
    Wait-Status 'awake' $true 'not awake after start 1'
    Assert-Requests 'start 1'

    Invoke-Wick 'stop' | Out-Null
    Wait-Until 'request left after stop' { Test-Released }
    Wait-Mode 'off' 'session still on after stop'
    Wait-Status 'awake' $false 'still awake after stop'

    if ($env:WICK_SMOKE_HOLDS -ne '0') { Test-Holds }
    Test-Console

    Invoke-Wick 'start' | Out-Null
    Wait-Mode 'indefinite' 'start is not open-ended'
    Wait-Status 'awake' $true 'not awake after start'
    Assert-Requests 'start'

    Stop-Process -Id $script:appPid -Force
    Wait-Process -Id $script:appPid -Timeout 15 -ErrorAction SilentlyContinue
    Wait-Until 'request left after a forced kill' { Test-Released }
    Wait-Status 'running' $false 'status still says running after a forced kill'
    Assert-Status 'awake' $false 'status still says awake after a forced kill'
}

$failed = $false
try {
    Test-Wick
    Write-Host 'Windows smoke test passed.'
} catch {
    $failed = $true
    Write-Host "Windows smoke test failed: $_"
    Write-Host '--- status'
    Write-Host ($script:status | ConvertTo-Json -Compress -Depth 5)
    Write-Host '--- Wick log'
    Get-Content -LiteralPath $log, $errorLog -ErrorAction SilentlyContinue | Write-Host
    Write-Host '--- powercfg /requests'
    powercfg /requests | Write-Host
} finally {
    if ($script:runner) {
        try { $script:runner.Process.Kill($true) } catch { }
    }
    $processes = @($script:appPid)
    if ($script:launched) { $processes += $script:launched.Id }
    foreach ($process in $processes) {
        if ($process) { Stop-Process -Id $process -Force -ErrorAction SilentlyContinue }
    }
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
if ($failed) { exit 1 }
