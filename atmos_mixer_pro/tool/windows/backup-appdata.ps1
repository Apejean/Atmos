# Atmos Windows 하네스: 앱 데이터(설정·환경설정·공연 상태), 로그, 로그인 자동 실행 키를 백업하거나 되돌린다.
# 백업:     powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Label before-E
# 되돌리기: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Restore D:\dev\backups\<폴더>
# 되돌리기는 사용자 확인 뒤에만 한다. 지금 상태를 먼저 백업하고, 앱 데이터 폴더를 백업 내용으로 바꾼다.
param([string]$Label = 'manual', [string]$Restore = '')
. (Join-Path $PSScriptRoot 'env.ps1')
$running = @(Get-Process -Name atmos_mixer_pro, atmos_supervisor -ErrorAction SilentlyContinue)
function New-Backup([string]$Tag) {
  $dest = Join-Path $Atmos.Backups ((Get-Date -Format 'yyyyMMdd-HHmmss') + "-$Tag")
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  if (Test-Path $Atmos.AppData) {
    Copy-Item -Path $Atmos.AppData -Destination (Join-Path $dest 'AppData') -Recurse -Force
  } else {
    Set-Content -Path (Join-Path $dest 'NO_APPDATA.txt') -Value $Atmos.AppData
  }
  if (Test-Path $Atmos.AppLogs) {
    Copy-Item -Path $Atmos.AppLogs -Destination (Join-Path $dest 'AppLogs') -Recurse -Force -ErrorAction SilentlyContinue
  }
  & reg.exe export 'HKCU\Software\Microsoft\Windows\CurrentVersion\Run' (Join-Path $dest 'HKCU_Run.reg') /y | Out-Null
  return $dest
}
if ($Restore) {
  if ($running.Count -gt 0) { Write-Output 'FAIL stop atmos_supervisor first, then atmos_mixer_pro'; exit 1 }
  $src = Join-Path $Restore 'AppData'
  if (-not (Test-Path $src)) { Write-Output "FAIL no AppData folder in $Restore"; exit 1 }
  $pre = New-Backup 'pre-restore'
  Write-Output "saved the current state to $pre"
  if (Test-Path $Atmos.AppData) { Remove-Item -Path $Atmos.AppData -Recurse -Force }
  New-Item -ItemType Directory -Force -Path (Split-Path $Atmos.AppData) | Out-Null
  Copy-Item -Path $src -Destination $Atmos.AppData -Recurse -Force
  Write-Output "RESTORED $src -> $($Atmos.AppData)"
  exit 0
}
if ($running.Count -gt 0) { Write-Output 'WARN Atmos is running; files may change during the copy' }
$d = New-Backup $Label
Write-Output "BACKUP=$d"
