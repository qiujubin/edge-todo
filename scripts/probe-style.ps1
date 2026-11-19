param([int]$TargetPid = 0)
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class StyleProbe {
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int idx);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
    [DllImport("user32.dll")] public static extern int GetWindowTextW(IntPtr h, [MarshalAs(UnmanagedType.LPWStr)] StringBuilder s, int n);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    public static void Run(uint pid) {
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p == pid) {
                var sb = new StringBuilder(64);
                GetWindowTextW(h, sb, 64);
                long style = GetWindowLong(h, -16);   // GWL_STYLE
                long ex = GetWindowLong(h, -20);      // GWL_EXSTYLE
                RECT wr; GetWindowRect(h, out wr);
                Console.WriteLine("hwnd={0} title='{1}' rect=({2},{3},{4},{5})",
                    h.ToInt64(), sb.ToString(), wr.L, wr.T, wr.R, wr.B);
                Console.WriteLine("  style=0x{0:X8} exstyle=0x{1:X8}", (uint)style, (uint)ex);
                Console.WriteLine("  WS_CAPTION={0} WS_THICKFRAME={1} WS_MINIMIZE={2} WS_MAXIMIZE={3} WS_VISIBLE={4} WS_POPUP={5}",
                    (style & 0x00C00000) != 0, (style & 0x00040000) != 0, (style & 0x20000000) != 0,
                    (style & 0x01000000) != 0, (style & 0x10000000) != 0, (style & 0x80000000u) != 0);
            }
            return true;
        }, IntPtr.Zero);
    }
}
"@
[StyleProbe]::SetProcessDPIAware() | Out-Null
[StyleProbe]::Run([uint32]$TargetPid)
