# Atmos Windows 하네스 G2: 개발 도구·설정을 점검해 표로 보여 준다. FAIL이 하나라도 있으면 exit 1.
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일>
. (Join-Path $PSScriptRoot 'env.ps1')
$rows = New-Object System.Collections.Generic.List[object]
function Add-Row($Check, $Status, $Detail) {
  $rows.Add([pscustomobject]@{ Check = $Check; Status = $Status; Detail = "$Detail" })
}
# 명령을 돌려 출력에서 $Pattern에 맞는 첫 줄을 돌려준다. 명령이 없거나 실패하면 $null.
function Find-Line($Exe, [string[]]$ArgList, $Pattern = '.') {
  if (-not (Get-Command $Exe -ErrorAction SilentlyContinue)) { return $null }
  $global:LASTEXITCODE = 0
  $out = @(& $Exe @ArgList 2>&1 | ForEach-Object { "$_" })
  if ($LASTEXITCODE -ne 0) { return $null }
  foreach ($l in $out) { if ($l -match $Pattern) { return $l.Trim() } }
  return $null
}
function Test-Tool($Check, $Exe, [string[]]$ArgList, $Pattern = '.', [switch]$Optional) {
  $v = Find-Line $Exe $ArgList $Pattern
  if ($v) { Add-Row $Check 'PASS' $v }
  elseif ($Optional) { Add-Row $Check 'WARN' 'not found (optional)' }
  else { Add-Row $Check 'FAIL' 'not found or failed' }
}

$os = Get-CimInstance Win32_OperatingSystem
Add-Row 'Windows' 'INFO' ('{0} {1} (build {2})' -f $os.Caption, $os.Version, $os.BuildNumber)
foreach ($spec in @(@{ Name = 'C'; Min = 10 }, @{ Name = 'D'; Min = 40 })) {
  $drv = Get-PSDrive -Name $spec.Name -ErrorAction SilentlyContinue
  if (-not $drv) { Add-Row "Disk $($spec.Name):" 'FAIL' 'drive not found'; continue }
  $gb = [math]::Round($drv.Free / 1GB, 1)
  $st = 'PASS'; if ($gb -lt $spec.Min) { $st = 'WARN' }
  Add-Row "Disk $($spec.Name): free" $st "$gb GB (recommended >= $($spec.Min) GB)"
}
$dm = (Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock' -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
$st = 'FAIL'; if ($dm -eq 1) { $st = 'PASS' }
Add-Row 'Developer Mode' $st "AllowDevelopmentWithoutDevLicense=$dm (Flutter plugins need symlinks)"
$lp = (Get-ItemProperty -LiteralPath 'HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem' -ErrorAction SilentlyContinue).LongPathsEnabled
Add-Row 'LongPathsEnabled' 'INFO' "$lp (user turns it on only if path-too-long errors appear)"

Test-Tool 'git' 'git' @('--version')
if (Get-Command git -ErrorAction SilentlyContinue) {
  $gl = git config --global core.longpaths
  $st = 'WARN'; if ($gl -eq 'true') { $st = 'PASS' }
  Add-Row 'git core.longpaths' $st "$gl"
  Add-Row 'git core.autocrlf' 'INFO' "$(git config core.autocrlf) (keep the Git for Windows default, same as CI)"
}
Test-Tool 'gh' 'gh' @('--version')
if (Get-Command gh -ErrorAction SilentlyContinue) {
  $global:LASTEXITCODE = 0
  gh auth status *> $null
  $st = 'FAIL'; if ($LASTEXITCODE -eq 0) { $st = 'PASS' }
  Add-Row 'gh auth' $st 'gh auth status'
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsPath = $null
if (Test-Path $vswhere) {
  $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
}
if ($vsPath) {
  $vsName = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property displayName
  $vsVer = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property catalog_productDisplayVersion
  Add-Row 'Visual Studio C++' 'PASS' "$vsName $vsVer @ $vsPath"
} else {
  Add-Row 'Visual Studio C++' 'FAIL' 'no Visual Studio with "Desktop development with C++"'
}
# env.ps1이 넣는 VS 개발자 환경. 없으면 asio-sys가 vcvarsall.bat을 C:\Program Files에서만 찾다 멈춘다.
$st = 'FAIL'; if ($env:VCINSTALLDIR) { $st = 'PASS' }
Add-Row 'VS developer env (asio-sys)' $st "VCINSTALLDIR=$env:VCINSTALLDIR"

foreach ($name in @('RUSTUP_HOME', 'CARGO_HOME', 'PUB_CACHE', 'LIBCLANG_PATH', 'CPAL_ASIO_DIR')) {
  $u = [Environment]::GetEnvironmentVariable($name, 'User')
  $st = 'FAIL'
  if ($u -and $u.StartsWith($Atmos.Root, [StringComparison]::OrdinalIgnoreCase)) { $st = 'PASS' }
  Add-Row "User env $name" $st "$u"
}
Test-Tool 'rustc' 'rustc' @('-V')
Test-Tool 'cargo' 'cargo' @('-V')
$tc = Find-Line 'rustup' @('show', 'active-toolchain')
$st = 'FAIL'; if ($tc -match 'msvc') { $st = 'PASS' }
Add-Row 'rustup toolchain (msvc)' $st "$tc"
Test-Tool 'cargo clippy' 'cargo' @('clippy', '-V') -Optional

$lib = Join-Path $env:LIBCLANG_PATH 'libclang.dll'
$st = 'FAIL'; if (Test-Path $lib) { $st = 'PASS' }
Add-Row 'libclang.dll (ASIO bindgen)' $st $lib
# asio-sys 0.2.6은 CPAL_ASIO_DIR 바로 아래의 common, host, host\pc만 쓴다. 한 단계 더 깊이 풀려 있으면 빌드가 실패한다.
$asioDir = $env:CPAL_ASIO_DIR
$asioMiss = @(@('common\asio.h', 'host\asiodrivers.h', 'host\pc') | Where-Object { -not (Test-Path (Join-Path $asioDir $_)) })
if ($asioMiss.Count -eq 0) {
  Add-Row 'ASIO SDK (CPAL_ASIO_DIR)' 'PASS' $asioDir
} else {
  $d = "missing under ${asioDir}: " + ($asioMiss -join ', ')
  if (Test-Path $asioDir) {
    $deeper = Get-ChildItem -Path $asioDir -Recurse -Filter 'asio.h' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($deeper) { $d += " (asio.h is at $($deeper.FullName); point CPAL_ASIO_DIR at the folder that holds common and host)" }
  }
  Add-Row 'ASIO SDK (CPAL_ASIO_DIR)' 'FAIL' $d
}

Test-Tool 'flutter' 'flutter' @('--version') '^Flutter \d'
$dl = Find-Line 'dart' @('--version') 'Dart SDK version'
$st = 'FAIL'
if ($dl -and $dl -match 'version: (\d+\.\d+\.\d+)') {
  if ([version]$Matches[1] -ge [version]'3.12.1') { $st = 'PASS' }
}
Add-Row 'dart >= 3.12.1 (pubspec)' $st "$dl"

$wv = $null
foreach ($k in @('HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
                 'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}')) {
  $pv = (Get-ItemProperty -LiteralPath $k -ErrorAction SilentlyContinue).pv
  if ($pv -and $pv -ne '0.0.0.0') { $wv = $pv; break }
}
$st = 'WARN'; $d = 'missing (the 3D viewer needs it, W1)'
if ($wv) { $st = 'PASS'; $d = $wv }
Add-Row 'WebView2 Runtime' $st $d
$iscc = Join-Path $Atmos.Root 'InnoSetup6\ISCC.exe'
$st = 'WARN'; if (Test-Path $iscc) { $st = 'PASS' }
Add-Row 'Inno Setup (installer only)' $st $iscc
$vc = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64' -ErrorAction SilentlyContinue
$d = 'not registered'
if ($vc) { $d = "Installed=$($vc.Installed) Version=$($vc.Version)" }
Add-Row 'VC++ runtime x64' 'INFO' $d
Test-Tool 'node (agentmemory MCP)' 'node' @('-v') -Optional
$ps = @(Get-Process -Name atmos_mixer_pro, atmos_supervisor -ErrorAction SilentlyContinue)
$d = 'none'
if ($ps.Count -gt 0) { $d = ($ps | ForEach-Object { '{0}#{1}' -f $_.ProcessName, $_.Id }) -join ', ' }
Add-Row 'Running Atmos' 'INFO' $d

$rows | Format-Table -AutoSize -Wrap | Out-String -Width 250
$fails = @($rows | Where-Object { $_.Status -eq 'FAIL' }).Count
Write-Output "FAIL=$fails"
exit ([int]($fails -gt 0))
