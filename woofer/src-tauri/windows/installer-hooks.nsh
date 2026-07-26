; Start a fresh installation in the signed-in user's desktop session.
; Tauri's updater owns restarts for /UPDATE installs, so do not start twice.
; The finish-page checkbox is deliberately unchecked: this hook is the one
; authoritative automatic-launch path, including when Windows UAC is disabled.
!define MUI_FINISHPAGE_RUN_NOTCHECKED

!macro NSIS_HOOK_POSTINSTALL
  ${If} $UpdateMode = 0
    nsis_tauri_utils::RunAsUser "$INSTDIR\${MAINBINARYNAME}.exe" ""
  ${EndIf}
!macroend
