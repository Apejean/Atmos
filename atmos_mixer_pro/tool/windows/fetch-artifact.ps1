# Atmos Windows 하네스 P5: CI("Build and Release")가 만든 Windows zip을 내려받아 풀고 내용을 확인한다.
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Pr 12   (또는 -Branch <이름>, -RunId <번호>)
# 결과: D:\dev\artifacts\<run id>\app\ 와 manifest.txt. 꼭 있어야 할 파일이 빠지면 exit 1.
param([int]$Pr = 0, [string]$Branch = '', [long]$RunId = 0, [string]$Repo = 'Apejean/Atmos')
. (Join-Path $PSScriptRoot 'env.ps1')
$head = ''
if ($Pr -gt 0) {
  $info = gh pr view $Pr --repo $Repo --json headRefName,headRefOid | Out-String | ConvertFrom-Json
  if (-not $info) { Write-Output "FAIL cannot read PR $Pr"; exit 1 }
  $Branch = $info.headRefName
  $head = $info.headRefOid
}
if ($RunId -eq 0) {
  if (-not $Branch) { Write-Output 'FAIL give -Pr, -Branch or -RunId'; exit 2 }
  $runs = gh run list --repo $Repo --workflow build_release.yml --branch $Branch --limit 30 --json databaseId,headSha,status,conclusion,createdAt,event | Out-String | ConvertFrom-Json
  $ok = @($runs | Where-Object { $_.status -eq 'completed' -and $_.conclusion -eq 'success' })
  if ($head) { $ok = @($ok | Where-Object { $_.headSha -eq $head }) }
  if ($ok.Count -eq 0) {
    Write-Output "FAIL no successful run for branch=$Branch head=$head. Recent runs:"
    foreach ($r in @($runs | Select-Object -First 5)) { Write-Output ('  {0} {1} {2} {3} {4}' -f $r.databaseId, $r.event, $r.status, $r.conclusion, $r.headSha) }
    exit 1
  }
  $RunId = $ok[0].databaseId
  $head = $ok[0].headSha
}
$dest = Join-Path $Atmos.Artifacts "$RunId"
$app = Join-Path $dest 'app'
if (-not (Test-Path (Join-Path $app 'atmos_mixer_pro.exe'))) {
  New-Item -ItemType Directory -Force -Path $dest | Out-Null
  gh run download $RunId --repo $Repo --name atmos_mixer_pro_windows.zip --dir $dest
  if ($LASTEXITCODE -ne 0) { Write-Output "FAIL gh run download $RunId (expired artifact or no access?)"; exit 1 }
  $zip = Get-ChildItem -Path $dest -Filter 'atmos_mixer_pro_windows.zip' -Recurse | Select-Object -First 1
  if (-not $zip) { Write-Output "FAIL zip not found under $dest"; exit 1 }
  Expand-Archive -Path $zip.FullName -DestinationPath $app -Force
}
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("run=$RunId repo=$Repo branch=$Branch head=$head fetched=$(Get-Date -Format o)")
$must = @(
  'atmos_mixer_pro.exe', 'rust_lib_atmos_mixer_pro.dll', 'flutter_windows.dll', 'atmos_supervisor.exe',
  'data\icudtl.dat', 'data\app.so',
  'data\flutter_assets\assets\3d_simulator\studio_engine.html', 'data\flutter_assets\assets\models\listener_head.glb'
)
$missing = 0
foreach ($f in $must) {
  $p = Join-Path $app $f
  if (Test-Path $p) {
    $h = (Get-FileHash -Path $p -Algorithm SHA256).Hash.Substring(0, 16)
    $lines.Add(('OK   {0}  {1} bytes  sha256:{2}' -f $f, (Get-Item $p).Length, $h))
  } else {
    $missing++
    $lines.Add("MISS $f")
  }
}
$exe = Get-Item (Join-Path $app 'atmos_mixer_pro.exe') -ErrorAction SilentlyContinue
if ($exe) {
  $vi = $exe.VersionInfo
  $lines.Add(('exe ProductVersion={0} FileVersion={1} CompanyName={2} ProductName={3}' -f $vi.ProductVersion, $vi.FileVersion, $vi.CompanyName, $vi.ProductName))
}
$lines | Set-Content -Path (Join-Path $dest 'manifest.txt') -Encoding UTF8
$lines | ForEach-Object { Write-Output $_ }
Write-Output "APP=$app"
exit ([int]($missing -gt 0))
