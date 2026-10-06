# Atmos Windows 하네스: 앱·감시 로그와 신호 파일, Windows 오류 이벤트·WER 보고서, 프로세스 상태를 증거 폴더로 모은다.
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Label 2-1_kill -Hours 2
# 결과: D:\dev\logs\<yyyy-MM-dd>\<Label>_<HHmmss>\
param([string]$Label = 'logs', [int]$Hours = 24)
. (Join-Path $PSScriptRoot 'env.ps1')
$dest = Join-Path (Join-Path $Atmos.Logs (Get-Date -Format 'yyyy-MM-dd')) ('{0}_{1}' -f $Label, (Get-Date -Format 'HHmmss'))
New-Item -ItemType Directory -Force -Path $dest | Out-Null
if (Test-Path $Atmos.AppLogs) {
  Copy-Item -Path (Join-Path $Atmos.AppLogs '*') -Destination $dest -Recurse -Force -ErrorAction SilentlyContinue
}
$since = (Get-Date).AddHours(-$Hours)
$events = @(Get-WinEvent -FilterHashtable @{ LogName = 'Application'; StartTime = $since } -ErrorAction SilentlyContinue |
  Where-Object { $_.Message -match 'atmos_(mixer_pro|supervisor)' })
$events | Select-Object TimeCreated, Id, ProviderName, LevelDisplayName, Message |
  Format-List | Out-String -Width 300 | Set-Content -Path (Join-Path $dest 'windows_events.txt') -Encoding UTF8
$wer = Join-Path $env:LOCALAPPDATA 'Microsoft\Windows\WER\ReportArchive'
if (Test-Path $wer) {
  foreach ($r in @(Get-ChildItem -Path $wer -Directory | Where-Object { $_.Name -match 'atmos' -and $_.LastWriteTime -ge $since })) {
    Copy-Item -Path $r.FullName -Destination (Join-Path $dest ('WER_' + $r.Name)) -Recurse -Force
  }
}
Get-Process -Name atmos_mixer_pro, atmos_supervisor -ErrorAction SilentlyContinue |
  Select-Object ProcessName, Id, StartTime,
    @{ n = 'WS_MB'; e = { [math]::Round($_.WorkingSet64 / 1MB, 1) } },
    @{ n = 'Private_MB'; e = { [math]::Round($_.PrivateMemorySize64 / 1MB, 1) } },
    HandleCount, @{ n = 'Threads'; e = { $_.Threads.Count } }, Path |
  Format-Table -AutoSize | Out-String -Width 300 | Set-Content -Path (Join-Path $dest 'processes.txt') -Encoding UTF8
Write-Output "EVIDENCE=$dest"
Write-Output "windows_events=$($events.Count)"
