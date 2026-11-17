Add-Type @"
using System.Runtime.InteropServices;
public class RestoreCur {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
"@
[RestoreCur]::SetProcessDPIAware() | Out-Null
[RestoreCur]::SetCursorPos(2000, 900) | Out-Null
Write-Output "cursor restored to (2000,900) physical"
