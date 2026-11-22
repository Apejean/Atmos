# Atmos Windows 하네스: 앱 데이터(설정·환경설정·공연 상태), 로그, 로그인 자동 실행 키를 백업하거나 되돌린다.
# 백업:     powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Label before-E
# 되돌리기: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Restore D:\dev\backups\<폴더>
# 되돌리기는 사용자 확인 뒤에만 한다. 지금 상태를 먼저 백업하고, 앱 데이터 폴더를 백업 내용으로 바꾼다.
# 앱 데이터 폴더가 바뀌었다(2026-11-22, %APPDATA%\com.example → %APPDATA%\The Awesome). 백업은 두 폴더를 다 담는다
# (AppData = 새 폴더, AppDataLegacy = 옛 폴더). 그 전에 만든 백업의 AppData는 옛 폴더 것이라 옛 폴더로 되돌린다.
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
  Set-Content -Path (Join-Path $dest 'AppData.path.txt') -Value $Atmos.AppData
  if (Test-Path $Atmos.AppDataLegacy) {
    Copy-Item -Path $Atmos.AppDataLegacy -Destination (Join-Path $dest 'AppDataLegacy') -Recurse -Force
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
  # 2026-11-22 전 백업(AppData.path.txt 없음)의 AppData는 옛 폴더 것이다.
  $pairs = @()
  if (Test-Path (Join-Path $Restore 'AppData.path.txt')) {
    $pairs += ,@($src, $Atmos.AppData)
    $legacySrc = Join-Path $Restore 'AppDataLegacy'
    if (Test-Path $legacySrc) { $pairs += ,@($legacySrc, $Atmos.AppDataLegacy) }
  } else {
    $pairs += ,@($src, $Atmos.AppDataLegacy)
  }
  $pre = New-Backup 'pre-restore'
  Write-Output "saved the current state to $pre"
  foreach ($pair in $pairs) {
    $from = $pair[0]; $to = $pair[1]
    if (Test-Path $to) { Remove-Item -Path $to -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Split-Path $to) | Out-Null
    Copy-Item -Path $from -Destination $to -Recurse -Force
    Write-Output "RESTORED $from -> $to"
  }
  exit 0
}
if ($running.Count -gt 0) { Write-Output 'WARN Atmos is running; files may change during the copy' }
$d = New-Backup $Label
Write-Output "BACKUP=$d"
