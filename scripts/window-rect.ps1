# Dev helper: print the Notch Desktop window's on-screen rectangle, style flags, and the primary monitor work area.
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class NotchWin {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int i);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
}
"@
Add-Type -AssemblyName System.Windows.Forms
$p = Get-Process notch-desktop -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Write-Output "no visible notch-desktop window"; exit 1 }
$h = $p.MainWindowHandle
$r = New-Object NotchWin+RECT
[void][NotchWin]::GetWindowRect($h, [ref]$r)
$ex = [NotchWin]::GetWindowLong($h, -20)
[pscustomobject]@{
  left = $r.Left; top = $r.Top; width = $r.Right - $r.Left; height = $r.Bottom - $r.Top
  visible = [NotchWin]::IsWindowVisible($h)
  topmost = [bool]($ex -band 0x8)
  toolWindow = [bool]($ex -band 0x80)
  dpi = [NotchWin]::GetDpiForWindow($h)
  workArea = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea.ToString()
} | Format-List
