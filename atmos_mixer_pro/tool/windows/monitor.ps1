# Atmos Windows 하네스: 장시간 시험 동안 앱·감시 프로세스의 자원을 CSV로 남긴다(점검표 4절).
# 시작(따로 떠 있는 창): Start-Process powershell -WindowStyle Minimized -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','D:\dev\Atmos\atmos_mixer_pro\tool\windows\monitor.ps1','-Hours','12'
# 멈춤: New-Item D:\dev\harness\monitor.stop  (또는 그 창을 닫는다)
# 결과: D:\dev\logs\monitor_<yyyyMMdd-HHmmss>.csv  (프로세스가 없으면 그 시각에 none 줄)
param([double]$Hours = 12, [int]$IntervalSec = 60, [string]$Out = '')
. (Join-Path $PSScriptRoot 'env.ps1')
New-Item -ItemType Directory -Force -Path $Atmos.Harness, $Atmos.Logs | Out-Null
$stop = Join-Path $Atmos.Harness 'monitor.stop'
if (Test-Path $stop) { Remove-Item -Path $stop }
if (-not $Out) { $Out = Join-Path $Atmos.Logs ('monitor_{0}.csv' -f (Get-Date -Format 'yyyyMMdd-HHmmss')) }
Set-Content -Path $Out -Value 'time,process,pid,start,cpu_s,ws_mb,private_mb,handles,threads' -Encoding UTF8
$inv = [System.Globalization.CultureInfo]::InvariantCulture
$end = (Get-Date).AddHours($Hours)
while ((Get-Date) -lt $end -and -not (Test-Path $stop)) {
  $now = Get-Date -Format 's'
  $procs = @(Get-Process -Name atmos_mixer_pro, atmos_supervisor -ErrorAction SilentlyContinue)
  if ($procs.Count -eq 0) { Add-Content -Path $Out -Value "$now,none,,,,,,," -Encoding UTF8 }
  foreach ($p in $procs) {
    $values = [object[]]@($now, $p.ProcessName, $p.Id, $p.StartTime, $p.TotalProcessorTime.TotalSeconds,
      ($p.WorkingSet64 / 1MB), ($p.PrivateMemorySize64 / 1MB), $p.HandleCount, $p.Threads.Count)
    $line = [string]::Format($inv, '{0},{1},{2},{3:s},{4:F1},{5:F1},{6:F1},{7},{8}', $values)
    Add-Content -Path $Out -Value $line -Encoding UTF8
  }
  Start-Sleep -Seconds $IntervalSec
}
Write-Output "MONITOR_CSV=$Out"
