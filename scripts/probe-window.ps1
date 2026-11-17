param([int]$TargetPid = 0)
Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class WinProbe {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern int GetWindowRgn(IntPtr h, IntPtr rgn);
    [DllImport("gdi32.dll")] public static extern int GetRgnBox(IntPtr rgn, out RECT r);
    [DllImport("gdi32.dll")] public static extern IntPtr CreateRectRgn(int a, int b, int c, int d);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out int val, int size);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    public static List<long> Found = new List<long>();
    public static void Run(uint pid) {
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid) {
                int cloaked; DwmGetWindowAttribute(h, 14, out cloaked, 4);
                RECT wr; GetWindowRect(h, out wr);
                IntPtr rgn = CreateRectRgn(0, 0, 0, 0);
                int t = GetWindowRgn(h, rgn);
                RECT rr; GetRgnBox(rgn, out rr);
                Console.WriteLine("hwnd={0} visible={1} cloaked={2} rect=({3},{4},{5},{6}) rgnType={7} rgnBox=({8},{9},{10},{11})",
                    h.ToInt64(), IsWindowVisibleCk(h) ? 1 : 0, cloaked, wr.L, wr.T, wr.R, wr.B, t, rr.L, rr.T, rr.R, rr.B);
            }
            return true;
        }, IntPtr.Zero);
    }
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    public static bool IsWindowVisibleCk(IntPtr h) { return IsWindowVisible(h); }
}
"@
[WinProbe]::SetProcessDPIAware() | Out-Null
[WinProbe]::Run([uint32]$TargetPid)
