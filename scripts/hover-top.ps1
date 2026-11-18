param([int]$PhysX = 1296, [int]$PhysY = 4)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class HoverTop {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, int d, UIntPtr e);
}
"@
[HoverTop]::SetProcessDPIAware() | Out-Null
[HoverTop]::SetCursorPos(1296, 300) | Out-Null
Start-Sleep -Milliseconds 150
[HoverTop]::SetCursorPos($PhysX, $PhysY) | Out-Null
Start-Sleep -Milliseconds 120
[HoverTop]::mouse_event(0x0001, 0, 1, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[HoverTop]::mouse_event(0x0001, 0, -1, 0, [UIntPtr]::Zero)
Write-Output "hovered top edge at ($PhysX,$PhysY)"
