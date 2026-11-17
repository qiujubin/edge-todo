param([int]$PhysX = 2540, [int]$PhysY = 800)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class HoverPhys {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, int d, UIntPtr e);
}
"@
[HoverPhys]::SetProcessDPIAware() | Out-Null
# 抖动：先离开窗口区域再进入，确保产生完整 mousemove 链
[HoverPhys]::SetCursorPos(2200, $PhysY) | Out-Null
Start-Sleep -Milliseconds 150
[HoverPhys]::SetCursorPos($PhysX, $PhysY) | Out-Null
Start-Sleep -Milliseconds 120
[HoverPhys]::mouse_event(0x0001, 1, 0, 0, [UIntPtr]::Zero)  # relative nudge
Start-Sleep -Milliseconds 80
[HoverPhys]::mouse_event(0x0001, -1, 0, 0, [UIntPtr]::Zero)
Write-Output "hovered at ($PhysX,$PhysY) physical"
