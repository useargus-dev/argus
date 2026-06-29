; Post-install: InstallPath registry, ARGUS_HOME env, bin\argus.exe shim, PATH (HKCU + HKLM).
; Per-machine NSIS install adds argus\bin to system PATH (Program Files).
; Tauri 2 bundles sidecars at $INSTDIR\lib\argus\ (beside the main exe, not under resources\).
; PATH helpers NEVER replace the entire machine PATH with only argus\bin.
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

  ; Always register for the installing user (works without admin elevation).
  WriteRegStr HKCU "Software\Argus" "InstallPath" "$INSTDIR"
  Push "$INSTDIR\bin"
  Call ArgusAddToUserPath

  WriteRegStr HKLM "Software\Argus" "InstallPath" "$INSTDIR"
  Push "$INSTDIR\bin"
  Call ArgusAddToMachinePath

  Push "$INSTDIR"
  Call ArgusSetArgusHomeEnv

  ; Verify PATH was applied (append-only should always succeed unless PATH is corrupt).
  StrCpy $R8 ""
  ReadRegStr $R9 HKCU "Environment" "Path"
  Push $R9
  Push "$INSTDIR\bin"
  Call ArgusStrStr
  Pop $R8
  StrCmp $R8 "" 0 path_ok
  ReadRegStr $R9 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
  Push $R9
  Push "$INSTDIR\bin"
  Call ArgusStrStr
  Pop $R8
  StrCmp $R8 "" 0 path_ok
  MessageBox MB_OK|MB_ICONEXCLAMATION "Argus installed but could not add $INSTDIR\bin to PATH.$\n$\nOpen PowerShell as Administrator and run:$\n  scripts\repair-windows-path.ps1$\n$\nOr add $INSTDIR\bin to PATH manually, then open a new terminal."
  Goto path_done
  path_ok:
  DetailPrint "PATH verified: $INSTDIR\bin"
  path_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  SetRegView 64
  Call un.ArgusRemoveArgusHomeEnv
  Push "$INSTDIR\bin"
  Call un.ArgusRemoveFromUserPath
  Push "$INSTDIR\bin"
  Call un.ArgusRemoveFromMachinePath
  DeleteRegKey HKCU "Software\Argus"
  DeleteRegKey HKLM "Software\Argus"
!macroend

; --- Install PATH helpers ---

Function ArgusStrStr
  ; Stack: haystack, needle -> returns index string or empty if not found
  Exch $R1 ; needle
  Exch
  Exch $R0 ; haystack
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

!macro ArgusPathHasSystem32Impl StrStrFn
  Exch $R1
  Push $R2
  StrCpy $R0 ""
  Push $R1
  Push "\System32\"
  Call ${StrStrFn}
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push "\system32\"
  Call ${StrStrFn}
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push "\SYSTEM32\"
  Call ${StrStrFn}
  Pop $R2
  StrCmp $R2 "" done
  found:
  StrCpy $R0 "1"
  done:
  Pop $R2
  Push $R0
  Pop $R1
!macroend

; Stack: path -> returns "1" on stack if path contains a System32 directory entry
Function ArgusPathHasSystem32
  Exch $R1
  Push $R2
  StrCpy $R0 ""
  ; Windows PATH entries usually end with \system32 before ; or end-of-string, not \system32\
  !insertmacro ArgusPathHasSystem32Impl "ArgusStrStr"
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\Windows\system32"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\Windows\System32"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\WINDOWS\system32"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\WINDOWS\System32"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push "\system32;"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push "\System32;"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Goto done
  found:
  StrCpy $R0 "1"
  done:
  Push $R0
  Pop $R1
FunctionEnd

Function un.ArgusPathHasSystem32
  Exch $R1
  Push $R2
  StrCpy $R0 ""
  !insertmacro ArgusPathHasSystem32Impl "un.ArgusStrStr"
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\Windows\system32"
  Call un.ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\Windows\System32"
  Call un.ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\WINDOWS\system32"
  Call un.ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push ":\WINDOWS\System32"
  Call un.ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push "\system32;"
  Call un.ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Push $R1
  Push "\System32;"
  Call un.ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 found
  Goto done
  found:
  StrCpy $R0 "1"
  done:
  Push $R0
  Pop $R1
FunctionEnd

Function un.ArgusStrStr
  ; Same as ArgusStrStr (uninstaller cannot call install functions)
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

; Abort write when PATH looks like installer error text, not directories.
; Stack: path -> returns "1" on stack if corrupt
Function ArgusPathLooksCorrupt
  Exch $R1
  StrCpy $R0 ""
  Push $R1
  Push " is running"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 corrupt
  Push $R1
  Push "Please close"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 corrupt
  Push $R1
  Push "try again"
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" done
  corrupt:
  StrCpy $R0 "1"
  done:
  Push $R0
  Pop $R1
FunctionEnd

; $R1 = Windows machine PATH baseline when registry/process PATH is unusable
Function ArgusDefaultMachinePathBase
  StrCpy $R1 "$WINDIR\system32;$WINDIR;$WINDIR\System32\Wbem;$WINDIR\System32\WindowsPowerShell\v1.0\;$WINDIR\System32\OpenSSH\"
FunctionEnd

Function un.ArgusDefaultMachinePathBase
  StrCpy $R1 "$WINDIR\system32;$WINDIR;$WINDIR\System32\Wbem;$WINDIR\System32\WindowsPowerShell\v1.0\;$WINDIR\System32\OpenSSH\"
FunctionEnd

; Backup registry PATH once before first modification (best-effort recovery).
Function ArgusBackupMachinePathIfNeeded
  ReadRegStr $R8 HKCU "Software\Argus" "PathBackupMachine"
  StrCmp $R8 "" 0 done
  ReadRegStr $R8 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
  WriteRegStr HKCU "Software\Argus" "PathBackupMachine" "$R8"
  done:
FunctionEnd

Function ArgusBackupUserPathIfNeeded
  ReadRegStr $R8 HKCU "Software\Argus" "PathBackupUser"
  StrCmp $R8 "" 0 done
  ReadRegStr $R8 HKCU "Environment" "Path"
  WriteRegStr HKCU "Software\Argus" "PathBackupUser" "$R8"
  done:
FunctionEnd

; Ensure machine PATH base includes System32 before appending argus.
; Input/output: $R1
Function ArgusEnsureMachinePathBase
  Push $R2
  Push $R3
  Push $R1
  Call ArgusPathHasSystem32
  Pop $R2
  StrCmp $R2 "" 0 base_ok

  ; Registry PATH missing System32 — use installer's process PATH if available.
  ReadEnvStr $R3 "Path"
  StrLen $R2 $R3
  StrCmp $R2 0 use_defaults
  StrCpy $R1 $R3
  Push $R1
  Call ArgusPathHasSystem32
  Pop $R2
  StrCmp $R2 "" 0 base_ok

  use_defaults:
  Call ArgusDefaultMachinePathBase
  DetailPrint "Warning: machine PATH missing System32; bootstrapping Windows defaults before adding argus"

  base_ok:
  Pop $R3
  Pop $R2
FunctionEnd

; Append $R0 to $R1 if not already present. Output in $R1.
Function ArgusAppendDirIfMissing
  Exch $R0 ; directory to add
  Exch
  Exch $R1 ; existing PATH
  Push $R2
  Push $R1
  Push $R0
  Call ArgusStrStr
  Pop $R2
  StrCmp $R2 "" 0 already
  StrCmp $R1 "" 0 +3
    StrCpy $R1 $R0
    Goto done
  StrCpy $R1 "$R1;$R0"
  already:
  done:
  Pop $R2
  Exch $R1
FunctionEnd

Function ArgusAddToUserPath
  Exch $R0 ; directory to add
  Push $R1
  Push $R2
  Call ArgusBackupUserPathIfNeeded
  ReadRegStr $R1 HKCU "Environment" "Path"
  Push $R1
  Call ArgusPathLooksCorrupt
  Pop $R2
  StrCmp $R2 "" 0 user_corrupt
  Push $R1
  Push $R0
  Call ArgusAppendDirIfMissing
  Pop $R1
  Push $R1
  Call ArgusPathLooksCorrupt
  Pop $R2
  StrCmp $R2 "" 0 user_corrupt
  WriteRegExpandStr HKCU "Environment" "Path" "$R1"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  DetailPrint "Added to user PATH: $R0"
  Goto user_done
  user_corrupt:
  DetailPrint "Warning: user PATH looks corrupt; skipped PATH update (InstallPath registry still set)"
  user_done:
  Pop $R2
  Pop $R1
  Exch $R0
FunctionEnd

Function ArgusAddToMachinePath
  Exch $R0 ; directory to add
  Push $R1
  Push $R2
  Call ArgusBackupMachinePathIfNeeded
  ReadRegStr $R1 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
  Push $R1
  Call ArgusPathLooksCorrupt
  Pop $R2
  StrCmp $R2 "" 0 machine_corrupt
  ; Append-only: never replace the machine PATH wholesale.
  Push $R1
  Push $R0
  Call ArgusAppendDirIfMissing
  Pop $R1
  Push $R1
  Call ArgusPathLooksCorrupt
  Pop $R2
  StrCmp $R2 "" 0 machine_corrupt
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" "$R1"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  DetailPrint "Added to machine PATH: $R0"
  Goto machine_done
  machine_corrupt:
  DetailPrint "Warning: machine PATH looks corrupt; skipped PATH update (InstallPath registry still set)"
  machine_done:
  Pop $R2
  Pop $R1
  Exch $R0
FunctionEnd

; --- ARGUS_HOME environment variable (HKCU + HKLM, matches Linux profile.d) ---

Function ArgusSetArgusHomeEnv
  Exch $R0 ; install directory
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

; --- Uninstall PATH helpers (un. prefix required) ---

Function un.ArgusPathJoin
  ; Stack: existing, segment -> joined path (semicolon-separated)
  Exch $R1 ; new segment
  Exch
  Exch $R0 ; existing
  StrCmp $R1 "" done
  StrCmp $R0 "" 0 +3
    StrCpy $R0 $R1
    Goto done
  StrCpy $R0 "$R0;$R1"
  done:
    Exch $R0
    Pop $R1
FunctionEnd

Function un.ArgusPathRemoveEntry
  ; Stack: path, entry -> path with entry removed
  Exch $R0 ; entry to remove
  Exch
  Exch $R1 ; full path
  Push $R2
  Push $R3
  Push $R4
  Push $R5
  StrCpy $R2 ""
  StrCpy $R3 ""
  StrCpy $R4 $R1
  loop:
    StrCpy $R5 $R4 1
    StrCmp $R5 "" write
    StrCmp $R5 ";" 0 +8
      StrCmp $R3 $R0 0 +4
        StrCpy $R3 ""
        Goto next
      Push $R2
      Push $R3
      Call un.ArgusPathJoin
      Pop $R2
      StrCpy $R3 ""
      Goto next
    StrCpy $R3 "$R3$R5"
  next:
    StrCpy $R4 $R4 "" 1
    Goto loop
  write:
    StrCmp $R3 $R0 0 +2
      StrCpy $R3 ""
    StrCmp $R3 "" done_join
    Push $R2
    Push $R3
    Call un.ArgusPathJoin
    Pop $R2
  done_join:
    StrCpy $R1 $R2
    Pop $R5
    Pop $R4
    Pop $R3
    Pop $R2
    Exch $R1
    Pop $R0
FunctionEnd

Function un.ArgusRemoveFromUserPath
  Exch $R0 ; directory to remove
  Push $R1
  ReadRegStr $R1 HKCU "Environment" "Path"
  StrCmp $R1 "" done
  Push $R1
  Push $R0
  Call un.ArgusPathRemoveEntry
  Pop $R1
  WriteRegExpandStr HKCU "Environment" "Path" "$R1"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  DetailPrint "Removed from user PATH: $R0"
  done:
  Pop $R1
  Exch $R0
FunctionEnd

Function un.ArgusRemoveFromMachinePath
  Exch $R0
  Push $R1
  ReadRegStr $R1 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path"
  StrCmp $R1 "" done
  Push $R1
  Push $R0
  Call un.ArgusPathRemoveEntry
  Pop $R1
  ; Never leave machine PATH empty or without System32 after uninstall.
  Push $R1
  Call un.ArgusPathHasSystem32
  Pop $R2
  StrCmp $R2 "" 0 write
  ReadRegStr $R8 HKCU "Software\Argus" "PathBackupMachine"
  StrCmp $R8 "" 0 use_backup
  Call un.ArgusDefaultMachinePathBase
  Goto write
  use_backup:
  StrCpy $R1 $R8
  write:
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "Path" "$R1"
  SendMessage ${HWND_BROADCAST} ${WM_WININICHANGE} 0 "STR:Environment" /TIMEOUT=5000
  DetailPrint "Removed from machine PATH: $R0"
  done:
  Pop $R1
  Exch $R0
FunctionEnd
