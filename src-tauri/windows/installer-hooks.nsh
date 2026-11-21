; The `wick` command for the NSIS installer: wick-cli.exe (a console program) copied to wick.com,
; and the install folder on the user's PATH. PATHEXT lists .COM before .EXE, so `wick` in cmd
; and PowerShell runs wick.com, not the GUI-subsystem wick.exe. Both go away on uninstall.
;
; PowerShell edits PATH because NSIS strings stop at 1024 characters and would cut a longer one.
; It reads the value unexpanded and keeps its type, so %VARIABLES% in it still work. The folder
; reaches it through the environment, so no quote in a path can break the script. Keep the
; command line itself under that limit.

!include WinMessages.nsh

!macro WICK_EDIT_PATH ACTION
  Push $0
  System::Call 'Kernel32::SetEnvironmentVariable(t "WICK_PATH_DIR", t "$INSTDIR")'
  System::Call 'Kernel32::SetEnvironmentVariable(t "WICK_PATH_ACTION", t "${ACTION}")'
  nsExec::Exec `"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -Command "$$dir = $$env:WICK_PATH_DIR.TrimEnd('\'); $$key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); $$exists = $$key.GetValueNames() -contains 'Path'; $$kind = if ($$exists) { $$key.GetValueKind('Path') } else { 'ExpandString' }; $$parts = @(([string]$$key.GetValue('Path', '', 'DoNotExpandEnvironmentNames')).Split(';') | Where-Object { $$_ }); $$others = @($$parts | Where-Object { $$_.TrimEnd('\') -ne $$dir }); $$add = $$env:WICK_PATH_ACTION -eq 'add'; if ($$add -and $$others.Count -lt $$parts.Count) { exit 0 }; if (-not $$add -and $$others.Count -eq $$parts.Count) { exit 0 }; if ($$add) { $$others += $$dir }; if ($$others.Count) { $$key.SetValue('Path', ($$others -join ';'), $$kind) } else { $$key.DeleteValue('Path', $$false) }"`
  Pop $0
  Pop $0
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  CopyFiles /SILENT "$INSTDIR\wick-cli.exe" "$INSTDIR\wick.com"
  !insertmacro WICK_EDIT_PATH add
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\wick.com"
  !insertmacro WICK_EDIT_PATH remove
!macroend
