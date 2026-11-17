#Requires -Version 7
# Runs a built Wick on Windows: it must start, hold a power request that powercfg lists under
# wick.exe while awake, report status, and let go on stop and on a forced kill.
# powercfg /requests needs an elevated shell; GitHub's Windows runners are elevated.
# Also checks wick hold, wick release and wick run; WICK_SMOKE_HOLDS=0 skips them.
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
    Get-Status | Out-Null
    Assert-Status 'awake' $true 'not awake while a hold works'
    Invoke-Wick 'release', 'smoke' | Out-Null
    Wait-Until 'request left after wick release' { Test-Released }
    Get-Status | Out-Null
    Assert-Status 'awake' $false 'still awake after wick release'

    $script:runner = Start-Wick 'run', '--', 'ping', '-n', '21', '127.0.0.1'
    Assert-Requests 'wick run'
    $result = Complete-Wick $script:runner
    $script:runner = $null
    if ($result.Code -ne 0) { throw "wick run exited with $($result.Code) for a command that succeeded: $($result.Err)" }
    Wait-Until "request left after wick run's command ended" { Test-Released }

    $result = Complete-Wick (Start-Wick 'run', '--', 'cmd', '/c', 'exit 7')
    if ($result.Code -ne 7) { throw "wick run exited with $($result.Code), expected the command's 7" }
    Write-Host 'holds: hold, release and run pass.'
}

function Test-Wick {
    $principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'powercfg /requests needs an elevated shell; run this from an administrator PowerShell.'
    }
    if (-not (Test-Path -LiteralPath $Bin -PathType Leaf)) { throw "no Wick binary at $Bin" }
    $script:Bin = (Resolve-Path -LiteralPath $Bin).Path

    $script:launched = Start-Process -FilePath $Bin -ArgumentList '--background' -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError $errorLog
    Wait-Until 'Wick did not start' { (Get-Status).running -eq $true } -Seconds 30
    $script:appPid = [int]$script:status.pid
    if ($script:appPid -ne $script:launched.Id) { throw "status names pid $($script:appPid), but Wick runs as $($script:launched.Id)" }
    Assert-Status 'awake' $false 'awake before any start'
    if (-not (Test-Released)) { throw "a request from wick.exe exists before any start: $(Get-WickRequests)" }

    Invoke-Wick 'start', '1' | Out-Null
    Get-Status | Out-Null
    Assert-Status 'awake' $true 'not awake after start 1'
    if ($script:status.session.mode -cne 'for') { throw "start 1 is not a timed session: $($script:status.session.mode)" }
    Assert-Requests 'start 1'

    Invoke-Wick 'stop' | Out-Null
    Wait-Until 'request left after stop' { Test-Released }
    Get-Status | Out-Null
    Assert-Status 'awake' $false 'still awake after stop'
    if ($script:status.session.mode -cne 'off') { throw "session still on after stop: $($script:status.session.mode)" }

    if ($env:WICK_SMOKE_HOLDS -ne '0') { Test-Holds }

    Invoke-Wick 'start' | Out-Null
    Get-Status | Out-Null
    Assert-Status 'awake' $true 'not awake after start'
    if ($script:status.session.mode -cne 'indefinite') { throw "start is not open-ended: $($script:status.session.mode)" }
    Assert-Requests 'start'

    Stop-Process -Id $script:appPid -Force
    Wait-Process -Id $script:appPid -Timeout 15 -ErrorAction SilentlyContinue
    Wait-Until 'request left after a forced kill' { Test-Released }
    Get-Status | Out-Null
    Assert-Status 'running' $false 'status still says running after a forced kill'
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
