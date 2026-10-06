# Atmos Windows 하네스: 빌드·장치 시험 전에 돌린다. 실행 중인 앱, 디스크 여유, 옛 dll, git 상태를 본다.
# FAIL이 있으면 exit 1(실행 중인 Atmos가 있으면 사용자에게 먼저 묻는다).
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> [-MinFreeC 5] [-MinFreeD 20]
param([int]$MinFreeC = 5, [int]$MinFreeD = 20)
. (Join-Path $PSScriptRoot 'env.ps1')
$fail = 0
$ps = @(Get-Process -Name atmos_mixer_pro, atmos_supervisor -ErrorAction SilentlyContinue)
if ($ps.Count -gt 0) {
  $fail++
  Write-Output 'FAIL Atmos is running. Ask the user before building or opening audio devices:'
  foreach ($p in $ps) { Write-Output ('  {0} pid={1} start={2} path={3}' -f $p.ProcessName, $p.Id, $p.StartTime, $p.Path) }
} else { Write-Output 'PASS no Atmos process' }
foreach ($spec in @(@{ Name = 'C'; Min = $MinFreeC }, @{ Name = 'D'; Min = $MinFreeD })) {
  $drv = Get-PSDrive -Name $spec.Name -ErrorAction SilentlyContinue
  if (-not $drv) { continue }
  $gb = [math]::Round($drv.Free / 1GB, 1)
  if ($gb -lt $spec.Min) { $fail++; Write-Output "FAIL $($spec.Name): free $gb GB < $($spec.Min) GB" }
  else { Write-Output "PASS $($spec.Name): free $gb GB" }
}
# 앱은 exe 옆 dll을 직접 연다(HANDOFF 13). 기본 로더를 쓰는 테스트(project_media_relink_test 등)는 이 파일을 먼저 열 수 있다.
$stale = Join-Path $Atmos.App 'rust\target\release\rust_lib_atmos_mixer_pro.dll'
if (Test-Path $stale) { Write-Output "WARN $stale exists ($((Get-Item $stale).LastWriteTime)); tests using the default FRB loader may load it" }
else { Write-Output 'PASS no rust\target\release dll' }
if (Test-Path (Join-Path $Atmos.Repo '.git')) {
  $branch = git -C $Atmos.Repo rev-parse --abbrev-ref HEAD
  $head = git -C $Atmos.Repo rev-parse --short HEAD
  $dirty = @(git -C $Atmos.Repo status --porcelain).Count
  Write-Output "INFO git $branch@$head, $dirty changed file(s)"
}
$lock = Join-Path $Atmos.AppLogs 'app.lock'
if (Test-Path $lock) { Write-Output "INFO $lock exists (app running, or it ended without a clean exit)" }
Write-Output "FAIL=$fail"
exit ([int]($fail -gt 0))
