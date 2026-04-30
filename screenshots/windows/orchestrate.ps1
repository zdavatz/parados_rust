# Drive parados through its menu and each game, capturing Microsoft
# Store screenshots at 1366x768 (same size as rust2xml /
# swissdamed2sqlite).
#
# Strategy mirrors screenshots/macos/capture.sh: launch parados once
# per shot with `--url <parados://...> --screenshot` so the deep-link
# loads the target game directly (no flaky click-simulation), then
# resize the window, capture, kill the process.  The two menu shots
# (01 + 02) launch without a --url so the menu renders, with one
# wheel-scroll between them.

param(
    [string]$Exe = "C:\Users\zdava\Documents\software\parados_rust\target\release\parados.exe",
    [int]$Width = 1366,
    [int]$Height = 768
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$signature = @'
using System;
using System.Runtime.InteropServices;
public static class Win32 {
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X; public int Y; }
}
'@
if (-not ('Win32' -as [type])) { Add-Type -TypeDefinition $signature }

function Get-GuiWindow {
    for ($i = 0; $i -lt 60; $i++) {
        $p = Get-Process -Name parados -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
        if ($p) { return $p }
        Start-Sleep -Milliseconds 500
    }
    throw "parados window did not appear within 30 seconds"
}

function Resize-Window([IntPtr]$hwnd, [int]$w, [int]$h) {
    [Win32]::ShowWindow($hwnd, 9) | Out-Null
    [Win32]::SetForegroundWindow($hwnd) | Out-Null
    # SWP_NOZORDER=0x0004, SWP_SHOWWINDOW=0x0040
    [Win32]::SetWindowPos($hwnd, [IntPtr]::Zero, 40, 40, $w, $h, 0x0044) | Out-Null
    Start-Sleep -Milliseconds 700
}

function Get-WindowRect([IntPtr]$hwnd) {
    $r = New-Object Win32+RECT
    [Win32]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    return $r
}

function Get-ClientOrigin([IntPtr]$hwnd) {
    $p = New-Object Win32+POINT
    [Win32]::ClientToScreen($hwnd, [ref]$p) | Out-Null
    return $p
}

function Capture([IntPtr]$hwnd, [string]$name) {
    $r = Get-WindowRect $hwnd
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, [System.Drawing.Size]::new($w, $h))
    $g.Dispose()
    $out = Join-Path $ScriptDir ("$name.png")
    $bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Saved $out  ($w x $h)"
}

function Move-Cursor([int]$x, [int]$y) {
    [Win32]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 60
}

function Wheel-Scroll([int]$screenX, [int]$screenY, [int]$ticks) {
    # ticks > 0 = wheel forward (scroll up).  ticks < 0 = scroll down.
    Move-Cursor $screenX $screenY
    # MOUSEEVENTF_WHEEL = 0x0800.  dwData is signed delta; pass via two's complement.
    $delta = $ticks * 120
    if ($delta -lt 0) { $delta = $delta + 4294967296 }
    [Win32]::mouse_event(0x0800, 0, 0, [uint32]$delta, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 250
}

function Launch-Parados([string[]]$ExtraArgs) {
    if ($ExtraArgs) {
        $gui = Start-Process -FilePath $Exe -ArgumentList $ExtraArgs -PassThru
    } else {
        $gui = Start-Process -FilePath $Exe -PassThru
    }
    $proc = Get-GuiWindow
    Resize-Window $proc.MainWindowHandle $Width $Height
    # Give the webview time to load + (when --screenshot is set) the
    # rules-dismiss helper time to fire its 350 + 900 ms timers.
    Start-Sleep -Milliseconds 2500
    return $gui
}

function Stop-Parados {
    Get-Process -Name parados -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 600
}

# Same deep-link URL set as screenshots/macos/capture.sh.  Launching
# with --url + --screenshot loads the game directly and auto-dismisses
# its rules modal — far more reliable than simulating button clicks.
$games = @(
    @{ name = "03-kangaroo";          url = "parados://localhost/games/kangaroo.html" },
    @{ name = "04-capovolto";         url = "parados://localhost/games/capovolto.html" },
    @{ name = "05-divided-loyalties"; url = "parados://localhost/games/divided_loyalties.html" },
    @{ name = "06-democracy";         url = "parados://localhost/games/democracy.html" },
    @{ name = "07-frankenstein";      url = "parados://localhost/games/frankenstein.html" },
    @{ name = "08-rainbow-blackjack"; url = "parados://localhost/games/rainbow_blackjack.html" },
    @{ name = "09-maka-laina";        url = "parados://localhost/games/makalaina.html" }
)

try {
    # --- Menu captures (single launch, no --url) ---
    $gui = Launch-Parados @()
    $hwnd = (Get-Process -Id $gui.Id).MainWindowHandle
    $client = Get-ClientOrigin $hwnd
    $scrollX = $client.X + 1200
    $scrollY = $client.Y + 400

    Capture $hwnd "01-menu"

    Wheel-Scroll $scrollX $scrollY -8
    Start-Sleep -Milliseconds 500
    Capture $hwnd "02-menu-scrolled"

    Stop-Parados

    # --- One game per fresh launch via --url + --screenshot ---
    foreach ($g in $games) {
        $gui = Launch-Parados @("--url", $g.url, "--screenshot")
        $hwnd = (Get-Process -Id $gui.Id).MainWindowHandle
        Capture $hwnd $g.name
        Stop-Parados
    }

    Write-Host "Done. Screenshots in $ScriptDir"
}
finally {
    Stop-Parados
}
