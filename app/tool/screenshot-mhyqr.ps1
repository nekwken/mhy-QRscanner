param(
  [string]$OutPath = "$env:TEMP\mhyqr-shot.png",
  [int]$ClickX = -1,
  [int]$ClickY = -1
)
$proc = Get-Process mhy_qrscanner | Select-Object -First 1
if (-not $proc) { Write-Output "mhy_qrscanner not running"; exit 1 }
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class QmdShot {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
  public struct RECT { public int Left, Top, Right, Bottom; }
}
'@
[QmdShot]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 400
if ($ClickX -ge 0) {
  $r0 = New-Object QmdShot+RECT
  [QmdShot]::GetWindowRect($proc.MainWindowHandle, [ref]$r0) | Out-Null
  [QmdShot]::SetCursorPos($r0.Left + $ClickX, $r0.Top + $ClickY) | Out-Null
  Start-Sleep -Milliseconds 150
  [QmdShot]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)  # left down
  [QmdShot]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)  # left up
  Start-Sleep -Milliseconds 900
}
$rect = New-Object QmdShot+RECT
[QmdShot]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null
$w = $rect.Right - $rect.Left
$h = $rect.Bottom - $rect.Top
Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[QmdShot]::PrintWindow($proc.MainWindowHandle, $hdc, 2) | Out-Null
$g.ReleaseHdc($hdc)
$g.Dispose()
$bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "captured ${w}x${h} -> $OutPath"
