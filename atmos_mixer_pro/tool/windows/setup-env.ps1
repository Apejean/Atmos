# Atmos Windows 개발 PC: 개발 도구 경로를 사용자 환경변수로 한 번 저장하고 하네스 폴더를 만든다.
# 다시 실행해도 결과가 같다. 저장한 값은 새로 띄운 프로그램부터 보인다(지금 셸에는 env.ps1을 점 소싱).
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> [-DevRoot D:\dev]
param([string]$DevRoot = 'D:\dev')
foreach ($d in @('downloads', 'harness', 'logs', 'artifacts', 'backups', 'sdk', 'rust')) {
  New-Item -ItemType Directory -Force -Path (Join-Path $DevRoot $d) | Out-Null
}
$vars = [ordered]@{
  RUSTUP_HOME   = (Join-Path $DevRoot 'rust\rustup')
  CARGO_HOME    = (Join-Path $DevRoot 'rust\cargo')
  PUB_CACHE     = (Join-Path $DevRoot 'pub-cache')
  LIBCLANG_PATH = (Join-Path $DevRoot 'LLVM\bin')
  CPAL_ASIO_DIR = (Join-Path $DevRoot 'sdk\asiosdk')
}
foreach ($k in $vars.Keys) {
  [Environment]::SetEnvironmentVariable($k, $vars[$k], 'User')
  Write-Output "set $k=$($vars[$k])"
}
# PATH는 %USERPROFILE% 같은 확장 문자열이 깨지지 않게 레지스트리 값 종류(REG_EXPAND_SZ)를 유지한 채 덧붙인다.
# ([Environment]::SetEnvironmentVariable로 PATH를 쓰면 REG_SZ가 되어 WindowsApps(winget) 경로가 깨질 수 있다.)
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
$old = [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
$parts = New-Object System.Collections.Generic.List[string]
foreach ($p in ($old -split ';')) { if ($p) { $parts.Add($p) } }
foreach ($rel in @('flutter\bin', 'rust\cargo\bin', 'InnoSetup6')) {
  $p = Join-Path $DevRoot $rel
  if (-not $parts.Contains($p)) { $parts.Add($p); Write-Output "PATH += $p" }
}
$key.SetValue('Path', ($parts -join ';'), [Microsoft.Win32.RegistryValueKind]::ExpandString)
$key.Close()
# 탐색기 등에 환경 변경을 알린다(SetEnvironmentVariable이 WM_SETTINGCHANGE를 보낸다).
[Environment]::SetEnvironmentVariable('ATMOS_DEV_ROOT', $DevRoot, 'User')
Write-Output 'done. Fully quit and reopen Claude and terminals to pick up the new variables.'
