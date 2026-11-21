#Requires -Version 7
# Installs the built NSIS setup, then the MSI, and checks the `wick` command each one sets up:
# wick-cli.exe and wick.com beside wick.exe, the install folder on PATH, and `wick` in cmd and
# PowerShell resolving to the console wick.com, which waits and returns the exit code. Then
# uninstalls and checks both are gone. Also unpacks the MSI the way Scoop does and checks
# wick-cli.exe, the target of Scoop's `wick` shim, is there.
# The MSI installs for all users, so this needs an elevated shell; GitHub's Windows runners are.
param(
    [Parameter(Mandatory)][string]$Setup,
    [Parameter(Mandatory)][string]$Msi
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$work = Join-Path ([IO.Path]::GetTempPath()) "wick-install-$PID"
New-Item -ItemType Directory -Force -Path $work | Out-Null

function Get-Subsystem([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($pe -lt 0 -or $pe + 94 -gt $bytes.Length -or [Text.Encoding]::ASCII.GetString($bytes, $pe, 4) -cne "PE`0`0") {
        throw "$Path is not a Windows program"
    }
    [BitConverter]::ToUInt16($bytes, $pe + 24 + 68)
}

function Wait-Program([string]$File, [string]$Arguments, [string]$What) {
    $process = Start-Process -FilePath $File -ArgumentList $Arguments -PassThru -WindowStyle Hidden
    if (-not $process.WaitForExit(300000)) { $process.Kill($true); throw "$What did not finish within 5 minutes" }
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "$What exited with $($process.ExitCode)" }
}

function Get-PathValue([string]$Scope) {
    $key = if ($Scope -eq 'User') {
        [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')
    } else {
        [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('SYSTEM\CurrentControlSet\Control\Session Manager\Environment')
    }
    if (-not $key) { return '' }
    [string]$key.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
}

function Test-OnPath([string]$Scope, [string]$Folder) {
    $folder = $Folder.TrimEnd('\')
    @((Get-PathValue $Scope).Split(';') | Where-Object { $_ -and $_.TrimEnd('\') -eq $folder }).Count -gt 0
}

# PATH as a new terminal would get it after the install.
function Get-NewPath {
    $paths = (Get-PathValue 'Machine'), (Get-PathValue 'User') | Where-Object { $_ }
    [Environment]::ExpandEnvironmentVariables($paths -join ';')
}

function Invoke-Cmd([string]$Line) {
    $info = [Diagnostics.ProcessStartInfo]::new("$env:SystemRoot\System32\cmd.exe")
    $info.Arguments = "/d /s /c `"$Line`""
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.Environment['Path'] = Get-NewPath
    $process = [Diagnostics.Process]::Start($info)
    $out = $process.StandardOutput.ReadToEndAsync()
    $err = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(30000)) { $process.Kill($true); throw "cmd /c $Line did not finish within 30s" }
    $process.WaitForExit()
    [pscustomobject]@{ Code = $process.ExitCode; Out = $out.Result; Err = $err.Result }
}

function Assert-Installed([string]$Folder, [string]$Scope, [string]$What) {
    foreach ($name in 'wick.exe', 'wick-cli.exe', 'wick.com') {
        if (-not (Test-Path -LiteralPath (Join-Path $Folder $name) -PathType Leaf)) { throw "${What}: no $name in $Folder" }
    }
    if ((Get-Subsystem (Join-Path $Folder 'wick.exe')) -ne 2) { throw "${What}: wick.exe is not a GUI program" }
    if ((Get-Subsystem (Join-Path $Folder 'wick.com')) -ne 3) { throw "${What}: wick.com is not a console program" }
    $cli = (Get-FileHash -LiteralPath (Join-Path $Folder 'wick-cli.exe')).Hash
    if ((Get-FileHash -LiteralPath (Join-Path $Folder 'wick.com')).Hash -ne $cli) { throw "${What}: wick.com differs from wick-cli.exe" }
    if (-not (Test-OnPath $Scope $Folder)) { throw "${What}: $Folder is not on the $Scope PATH: $(Get-PathValue $Scope)" }

    $saved = $env:Path
    try {
        $env:Path = Get-NewPath
        $found = (Get-Command wick -CommandType Application | Select-Object -First 1).Source
    } finally {
        $env:Path = $saved
    }
    $expected = [IO.Path]::GetFullPath((Join-Path $Folder 'wick.com'))
    if (-not $found -or [IO.Path]::GetFullPath($found) -ne $expected) { throw "${What}: PowerShell runs $found for wick, expected wick.com" }
    $result = Invoke-Cmd 'wick run -- cmd /c exit 7'
    if ($result.Code -ne 7) { throw "${What}: cmd /c wick run exited with $($result.Code), expected 7: $($result.Err)" }
    $result = Invoke-Cmd 'wick status'
    if ($result.Code -ne 0 -or $result.Out -notmatch '"running":') { throw "${What}: cmd /c wick status printed $($result.Out) (exit $($result.Code))" }
    Write-Host "${What}: wick in cmd and PowerShell is $found; run returns 7; status prints."
}

function Assert-Removed([string]$Folder, [string]$Scope, [string]$What) {
    if (Test-Path -LiteralPath (Join-Path $Folder 'wick.com')) { throw "${What}: wick.com is still in $Folder" }
    if (Test-OnPath $Scope $Folder) { throw "${What}: $Folder is still on the $Scope PATH" }
    Write-Host "${What}: uninstall removed wick.com and the PATH entry."
}

$failed = $false
try {
    $Setup = (Resolve-Path -LiteralPath $Setup).Path
    $Msi = (Resolve-Path -LiteralPath $Msi).Path

    $folder = Join-Path $env:LOCALAPPDATA 'Wick'
    Wait-Program $Setup '/S' 'NSIS install'
    Assert-Installed $folder 'User' 'NSIS'
    Wait-Program (Join-Path $folder 'uninstall.exe') "/S _?=$folder" 'NSIS uninstall'
    Assert-Removed $folder 'User' 'NSIS'
    Remove-Item -LiteralPath $folder -Recurse -Force -ErrorAction SilentlyContinue

    # The MSI would reuse the folder the NSIS setup recorded under HKCU\Software\abulkosim\Wick.
    $folder = Join-Path $env:ProgramFiles 'Wick'
    Wait-Program 'msiexec.exe' "/i `"$Msi`" /qn /norestart INSTALLDIR=`"$folder\`" /l*v `"$work\install.log`"" 'MSI install'
    Assert-Installed $folder 'Machine' 'MSI'
    Wait-Program 'msiexec.exe' "/x `"$Msi`" /qn /norestart /l*v `"$work\uninstall.log`"" 'MSI uninstall'
    Assert-Removed $folder 'Machine' 'MSI'

    $unpacked = Join-Path $work 'unpacked'
    Wait-Program 'msiexec.exe' "/a `"$Msi`" /qn TARGETDIR=`"$unpacked`"" 'MSI unpack'
    $cli = Join-Path $unpacked 'PFiles\Wick\wick-cli.exe'
    if (-not (Test-Path -LiteralPath $cli -PathType Leaf) -or (Get-Subsystem $cli) -ne 3) { throw "Scoop: no console wick-cli.exe at $cli" }
    Write-Host 'Scoop: the unpacked MSI has wick-cli.exe for the wick shim.'
    Write-Host 'Windows install check passed.'
} catch {
    $failed = $true
    Write-Host "Windows install check failed: $_"
    Get-ChildItem -LiteralPath $work -Filter '*.log' -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host "--- $($_.Name) (failed actions)"
        Select-String -LiteralPath $_.FullName -Pattern 'return value 3' -Context 20, 0 | Select-Object -First 40 | ForEach-Object { $_.Context.PreContext + $_.Line | Write-Host }
    }
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
if ($failed) { exit 1 }
