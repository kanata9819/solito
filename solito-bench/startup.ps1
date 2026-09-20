param(
    [string]$Executable = "$PSScriptRoot/../target/release/solito.exe",
    [ValidateRange(1, 100)][int]$Runs = 5
)

$ErrorActionPreference = 'Stop'
if (-not ('SolitoStartupWindow' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class SolitoStartupWindow {
    delegate bool Callback(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback, IntPtr data);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr window, StringBuilder text, int count);
    public static bool IsReady(int pid) {
        bool ready = false;
        EnumWindows((window, data) => {
            GetWindowThreadProcessId(window, out uint owner);
            if (owner != pid || !IsWindowVisible(window)) return true;
            var title = new StringBuilder(256);
            GetWindowText(window, title, title.Capacity);
            ready = title.ToString() == "Solito";
            return !ready;
        }, IntPtr.Zero);
        return ready;
    }
}
'@
}
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$samples = for ($run = 1; $run -le $Runs; $run++) {
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new($executablePath)
    $startInfo.UseShellExecute = $false
    $startInfo.Environment['SOLITO_SHELL_PROGRAM'] = $env:ComSpec
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $process = [System.Diagnostics.Process]::Start($startInfo)
    try {
        # Ignore winit's helper window; Solito shows the named window after drawing.
        do {
            $process.Refresh()
            if ($process.HasExited) { throw 'Solito exited before showing its window.' }
            if ($timer.Elapsed.TotalSeconds -gt 15) { throw 'Startup exceeded 15 seconds.' }
            if ([SolitoStartupWindow]::IsReady($process.Id)) { break }
            Start-Sleep -Milliseconds 5
        } while ($true)
        $timer.Stop()
        $timer.Elapsed.TotalMilliseconds
    } finally {
        if (-not $process.HasExited) {
            $process.Kill()
            $process.WaitForExit()
        }
        $process.Dispose()
    }
}
$ordered = @($samples | Sort-Object)
$middle = [int][Math]::Floor($Runs / 2)
$median = if ($Runs % 2) { $ordered[$middle] } else { ($ordered[$middle - 1] + $ordered[$middle]) / 2 }
[pscustomobject]@{
    Executable = $executablePath
    SamplesMs = @($samples | ForEach-Object { [Math]::Round($_, 2) })
    MedianMs = [Math]::Round($median, 2)
}
