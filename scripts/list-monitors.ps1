Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Screen]::AllScreens | ForEach-Object {
    Write-Output ("{0} Primary={1} Bounds={2}" -f $_.DeviceName, $_.Primary, $_.Bounds.ToString())
}
