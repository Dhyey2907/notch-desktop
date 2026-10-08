; Overline NSIS installer hooks.

!macro NSIS_HOOK_POSTUNINSTALL
  ; "Start with Windows" is stored per user by the app, not by the installer, so remove it
  ; here or Windows would keep trying to launch a program that no longer exists. (The app was
  ; called Notch Desktop before, so clear that name too.)
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Overline"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Notch Desktop"
!macroend
