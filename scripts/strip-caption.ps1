param([int]$TargetPid = 0)
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class StripTest {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr h, int idx);
    [DllImport("user32.dll")] public static extern int SetWindowLongW(IntPtr h, int idx, int v);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
    public static IntPtr Found = IntPtr.Zero;
    public static void Find(uint pid) {
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid) { Found = h; return false; }
            return true;
        }, IntPtr.Zero);
    }
    public static string Strip() {
        long style = GetWindowLongW(Found, -16);
        long cleaned = style & ~0x00C00000L & ~0x00040000L;
        int r1 = SetWindowLongW(Found, -16, (int)cleaned);
        SetWindowPos(Found, IntPtr.Zero, 0, 0, 0, 0, 0x0027);
        long after = GetWindowLongW(Found, -16);
        return string.Format("before=0x{0:X8} setRet={1} after=0x{2:X8} captionNow={3}",
            (uint)style, r1, (uint)after, (after & 0x00C00000L) != 0);
    }
}
"@
[StripTest]::SetProcessDPIAware() | Out-Null
[StripTest]::Find([uint32]$TargetPid)
if ([StripTest]::Found -eq [IntPtr]::Zero) { Write-Output "not found"; exit }
Write-Output ([StripTest]::Strip())
