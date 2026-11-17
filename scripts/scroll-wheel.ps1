param([int]$Notches = 3)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Wheel {
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, int dwData, UIntPtr dwExtraInfo);
    public const uint WHEEL = 0x0800;
}
"@
[Wheel]::mouse_event([Wheel]::WHEEL, 0, 0, -120 * $Notches, [UIntPtr]::Zero)
Write-Output "wheeled -$Notches"
