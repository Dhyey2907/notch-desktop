; Notch Desktop NSIS installer hooks.

!macro NSIS_HOOK_POSTUNINSTALL
  ; "Start with Windows" is stored per user by the app, not by the installer, so remove it
  ; here or Windows would keep trying to launch a program that no longer exists.
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Notch Desktop"
!macroend
