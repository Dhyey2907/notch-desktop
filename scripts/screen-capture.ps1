# Dev helper: capture real screen pixels around the Notch Desktop window (shows true transparency/shadow).
# Usage: powershell -File scripts/screen-capture.ps1 <out.png> [padding]
param([string]$Out = "notch-screen.png", [int]$Padding = 40)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class NotchCap {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
Add-Type -AssemblyName System.Drawing
[void][NotchCap]::SetProcessDPIAware()
$p = Get-Process notch-desktop -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$r = New-Object NotchCap+RECT
[void][NotchCap]::GetWindowRect($p.MainWindowHandle, [ref]$r)
$x = [Math]::Max(0, $r.Left - $Padding); $y = [Math]::Max(0, $r.Top)
$w = ($r.Right - $r.Left) + 2 * $Padding; $h = ($r.Bottom - $r.Top) + $Padding
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Output "$Out ($w x $h at $x,$y)"
