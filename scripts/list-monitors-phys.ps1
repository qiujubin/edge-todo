Add-Type @"
using System.Runtime.InteropServices;
public class Dpi { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
[Dpi]::SetProcessDPIAware() | Out-Null
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Screen]::AllScreens | ForEach-Object {
    Write-Output ("{0} Primary={1} PhysicalBounds={2}" -f $_.DeviceName, $_.Primary, $_.Bounds.ToString())
}
