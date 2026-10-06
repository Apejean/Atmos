# Atmos Windows 하네스: 폴더 안 exe·dll이 기대는 DLL을 dumpbin /dependents로 보고,
# 깨끗한 PC에 없을 수 있는 VC++ 런타임(VCRUNTIME140*.dll, MSVCP140*.dll) 의존을 표시한다(X1).
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Dir D:\dev\artifacts\<run>\app
param([string]$Dir = '')
. (Join-Path $PSScriptRoot 'env.ps1')
if (-not $Dir -or -not (Test-Path $Dir)) { Write-Output 'usage: deps.ps1 -Dir <folder with exe/dll>'; exit 2 }
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = $null
if (Test-Path $vswhere) { $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath }
if (-not $vs) { Write-Output 'FAIL Visual Studio with C++ tools not found (dumpbin comes with it)'; exit 1 }
$msvc = Get-ChildItem -Path (Join-Path $vs 'VC\Tools\MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$dumpbin = Join-Path $msvc.FullName 'bin\Hostx64\x64\dumpbin.exe'
if (-not (Test-Path $dumpbin)) { Write-Output "FAIL dumpbin not found: $dumpbin"; exit 1 }
$needVc = 0
foreach ($f in @(Get-ChildItem -Path $Dir -File | Where-Object { $_.Extension -in '.exe', '.dll' })) {
  $deps = @(& $dumpbin /nologo /dependents $f.FullName | ForEach-Object { "$_".Trim() } | Where-Object { $_ -match '^[\w\.\-]+\.dll$' })
  $vc = @($deps | Where-Object { $_ -match '^(vcruntime|msvcp|concrt|vccorlib)\d' })
  $tag = 'OK'
  if ($vc.Count -gt 0) { $tag = 'VC'; $needVc++ }
  Write-Output ('{0}  {1}: {2}' -f $tag, $f.Name, ($deps -join ', '))
}
Write-Output "files_needing_vc_runtime=$needVc"
