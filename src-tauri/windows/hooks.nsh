; Post-install: InstallPath registry, ARGUS_HOME, App Paths, user shim.
; Does NOT read or write the Path environment variable (avoids PATH corruption).
; Tauri 2 bundles sidecars at $INSTDIR\lib\argus\ (beside the main exe, not under resources\).
; Uninstall helpers must use the un. prefix (NSIS requirement in uninstall sections).

!include "WinMessages.nsh"

!macro NSIS_HOOK_POSTINSTALL
  SetRegView 64

  CreateDirectory "$INSTDIR\bin"

  ; Upgrade from pre-0.3.1 installs that bundled extensionless sidecars.
  ${If} ${FileExists} "$INSTDIR\lib\argus\argus-cli"
  ${AndIfNot} ${FileExists} "$INSTDIR\lib\argus\argus-cli.exe"
    CopyFiles /SILENT "$INSTDIR\lib\argus\argus-cli" "$INSTDIR\lib\argus\argus-cli.exe"
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\lib\argus\argus-redirector-windows"
  ${AndIfNot} ${FileExists} "$INSTDIR\lib\argus\argus-redirector-windows.exe"
    CopyFiles /SILENT "$INSTDIR\lib\argus\argus-redirector-windows" "$INSTDIR\lib\argus\argus-redirector-windows.exe"
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\lib\argus\argus-cli.exe"
    Delete "$INSTDIR\lib\argus\argus-cli"
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\lib\argus\argus-redirector-windows.exe"
    Delete "$INSTDIR\lib\argus\argus-redirector-windows"
  ${EndIf}

  ; Require full sidecar stack (matches Linux postinst).
  StrCpy $R9 ""
  ${If} ${FileExists} "$INSTDIR\lib\argus\argus-cli.exe"
    StrCpy $R9 "1"
  ${ElseIf} ${FileExists} "$INSTDIR\resources\lib\argus\argus-cli.exe"
    StrCpy $R9 "1"
  ${EndIf}
  StrCmp $R9 "1" cli_bundle_ok
    MessageBox MB_OK|MB_ICONSTOP "Argus install incomplete: CLI sidecar missing.$\n$\nDownload and run the full setup.exe installer from GitHub releases."
    Abort
  cli_bundle_ok:

  ; CLI sidecar -> bin\argus.exe for terminal use
  ${If} ${FileExists} "$INSTDIR\lib\argus\argus-cli.exe"
    CopyFiles /SILENT "$INSTDIR\lib\argus\argus-cli.exe" "$INSTDIR\bin\argus.exe"
  ${Else}
    CopyFiles /SILENT "$INSTDIR\resources\lib\argus\argus-cli.exe" "$INSTDIR\bin\argus.exe"
  ${EndIf}

  StrCpy $R9 ""
  ${If} ${FileExists} "$INSTDIR\lib\argus\argus-redirector-windows.exe"
    StrCpy $R9 "1"
  ${ElseIf} ${FileExists} "$INSTDIR\resources\lib\argus\argus-redirector-windows.exe"
    StrCpy $R9 "1"
  ${EndIf}
  StrCmp $R9 "1" redir_bundle_ok
    MessageBox MB_OK|MB_ICONSTOP "Argus install incomplete: redirector missing.$\n$\nDownload and run the full setup.exe installer from GitHub releases."
    Abort
  redir_bundle_ok:

  ${IfNot} ${FileExists} "$INSTDIR\lib\argus\WinDivert.dll"
    MessageBox MB_OK|MB_ICONSTOP "Argus install incomplete: WinDivert.dll missing.$\n$\nDownload and run the full setup.exe installer from GitHub releases."
    Abort
  ${EndIf}
  ${IfNot} ${FileExists} "$INSTDIR\lib\argus\WinDivert64.sys"
    MessageBox MB_OK|MB_ICONSTOP "Argus install incomplete: WinDivert64.sys missing.$\n$\nDownload and run the full setup.exe installer from GitHub releases."
    Abort
  ${EndIf}

  WriteRegStr HKCU "Software\Argus" "InstallPath" "$INSTDIR"
  WriteRegStr HKLM "Software\Argus" "InstallPath" "$INSTDIR"

  Push "$INSTDIR"
  Call ArgusSetArgusHomeEnv

  Push "$INSTDIR\bin\argus.exe"
  Call ArgusRegisterAppPath

  Call ArgusInstallUserShim
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  SetRegView 64
  Call un.ArgusRemoveArgusHomeEnv
  Call un.ArgusUnregisterAppPath
  Call un.ArgusRemoveUserShim
  DeleteRegKey HKCU "Software\Argus"
  DeleteRegKey HKLM "Software\Argus"
!macroend

Function ArgusSetArgusHomeEnv
  Exch $R0
  WriteRegExpandStr HKCU "Environment" "ARGUS_HOME" "$R0"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "ARGUS_HOME" "$R0"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  DetailPrint "Set ARGUS_HOME=$R0 (user + machine)"
  Exch $R0
FunctionEnd

Function un.ArgusRemoveArgusHomeEnv
  DeleteRegValue HKCU "Environment" "ARGUS_HOME"
  DeleteRegValue HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "ARGUS_HOME"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  DetailPrint "Removed ARGUS_HOME"
FunctionEnd

; Register argus.exe via App Paths (no Path env var changes).
Function ArgusRegisterAppPath
  Exch $R0 ; full path to argus.exe
  Push $R1
  StrCpy $R1 "$R0"
  ; Strip filename for the optional Path value (DLL search path).
  Push $R1
  Push "\"
  Call ArgusStrRStr
  Pop $R2
  StrCpy $R1 $R1 $R2
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\argus.exe" "" "$R0"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\argus.exe" "Path" "$R1"
  DetailPrint "Registered App Paths: $R0"
  Pop $R1
  Exch $R0
FunctionEnd

Function un.ArgusUnregisterAppPath
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\argus.exe"
  DetailPrint "Removed App Paths registration"
FunctionEnd

; argus.cmd in %LOCALAPPDATA%\Microsoft\WindowsApps (already on typical user Path).
Function ArgusInstallUserShim
  Push $R0
  Push $R1
  ReadEnvStr $R0 "LOCALAPPDATA"
  StrCmp $R0 "" shim_done
  CreateDirectory "$R0\Microsoft\WindowsApps"
  StrCpy $R1 "$R0\Microsoft\WindowsApps\argus.cmd"
  FileOpen $R0 $R1 w
  FileWrite $R0 "@echo off$\r$\n"
  FileWrite $R0 '"$INSTDIR\bin\argus.exe" %*$\r$\n'
  FileClose $R0
  DetailPrint "Installed CLI shim: $R1"
  shim_done:
  Pop $R1
  Pop $R0
FunctionEnd

Function un.ArgusRemoveUserShim
  Push $R0
  ReadEnvStr $R0 "LOCALAPPDATA"
  StrCmp $R0 "" shim_done
  Delete "$R0\Microsoft\WindowsApps\argus.cmd"
  DetailPrint "Removed CLI shim"
  shim_done:
  Pop $R0
FunctionEnd

; haystack on stack, needle pushed — returns index or empty.
Function ArgusStrStr
  Exch $R1
  Exch
  Exch $R0
  Push $R2
  Push $R3
  StrLen $R2 $R1
  StrCpy $R3 0
  loop:
    StrCpy $R4 $R0 $R2 $R3
    StrCmp $R4 $R1 found
    StrCmp $R4 "" notfound
    IntOp $R3 $R3 + 1
    Goto loop
  found:
    StrCpy $R0 $R3
    Goto done
  notfound:
    StrCpy $R0 ""
  done:
    Pop $R3
    Pop $R2
    Pop $R1
    Exch $R0
FunctionEnd

; haystack on stack, needle pushed — last match index or empty.
Function ArgusStrRStr
  Exch $R1
  Exch
  Exch $R0
  Push $R2
  Push $R3
  Push $R4
  StrLen $R2 $R1
  StrCpy $R3 0
  StrCpy $R4 ""
  loop:
    StrCpy $R5 $R0 $R2 $R3
    StrCmp $R5 $R1 0 +3
      StrCpy $R4 $R3
      Goto advance
    StrCmp $R5 "" done
  advance:
    IntOp $R3 $R3 + 1
    Goto loop
  done:
    StrCpy $R0 $R4
    Pop $R4
    Pop $R3
    Pop $R2
    Pop $R1
    Exch $R0
FunctionEnd

Function un.ArgusStrStr
  Exch $R1
  Exch
  Exch $R0
  Push $R2
  Push $R3
  StrLen $R2 $R1
  StrCpy $R3 0
  loop:
    StrCpy $R4 $R0 $R2 $R3
    StrCmp $R4 $R1 found
    StrCmp $R4 "" notfound
    IntOp $R3 $R3 + 1
    Goto loop
  found:
    StrCpy $R0 $R3
    Goto done
  notfound:
    StrCpy $R0 ""
  done:
    Pop $R3
    Pop $R2
    Pop $R1
    Exch $R0
FunctionEnd
