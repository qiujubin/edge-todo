param(
    [switch]$Restore
)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Mouse {
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
    public struct POINT { public int X; public int Y; }
}
"@
if ($Restore) {
    $saved = Get-Content "E:\WorkSpace\zai\edge-todo\scripts\out\cursor.json" | ConvertFrom-Json
    [Mouse]::SetCursorPos($saved.X, $saved.Y) | Out-Null
    Write-Output "restored to ($($saved.X),$($saved.Y))"
    exit
}
$p = New-Object Mouse+POINT
[Mouse]::GetCursorPos([ref]$p) | Out-Null
@{ X = $p.X; Y = $p.Y } | ConvertTo-Json | Set-Content "E:\WorkSpace\zai\edge-todo\scripts\out\cursor.json"
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
# 先挪开再贴边，确保产生 mousemove 事件链
$x = $b.Width - 3
$y = [int]($b.Height * 0.5)
[Mouse]::SetCursorPos(($b.Width - 300), $y) | Out-Null
Start-Sleep -Milliseconds 120
[Mouse]::SetCursorPos($x, $y) | Out-Null
Write-Output "cursor moved to ($x,$y), bounds $($b.Width)x$($b.Height), was ($($p.X),$($p.Y))"
