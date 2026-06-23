; Post-install: registry InstallPath + CLI on PATH as bin\argus.exe
!macro NSIS_HOOK_POSTINSTALL
  SetRegView 64
  WriteRegStr HKLM "Software\Argus" "InstallPath" "$INSTDIR"

  CreateDirectory "$INSTDIR\bin"

  ; CLI sidecar (built as argus-cli) -> bin\argus.exe for terminal use
  ${If} ${FileExists} "$INSTDIR\resources\lib\argus\argus-cli.exe"
    CopyFiles /SILENT "$INSTDIR\resources\lib\argus\argus-cli.exe" "$INSTDIR\bin\argus.exe"
  ${ElseIf} ${FileExists} "$INSTDIR\resources\lib\argus\argus-cli"
    CopyFiles /SILENT "$INSTDIR\resources\lib\argus\argus-cli" "$INSTDIR\bin\argus.exe"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  SetRegView 64
  DeleteRegKey HKLM "Software\Argus"
!macroend
