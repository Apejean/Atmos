# Atmos Windows 하네스 W4: 설치 파일에 넣을 Microsoft 재배포 파일을 받는다.
#   VC_redist.x64.exe                              Visual C++ 런타임(최신 지원판, aka.ms/vc14)
#   MicrosoftEdgeWebView2RuntimeInstallerX64.exe   WebView2 런타임 Evergreen Standalone(오프라인 설치용)
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> [-Force]
# 결과: atmos_mixer_pro\windows\redist\ 에 두 파일과 redist.txt(크기·버전·SHA256·서명).
#       Microsoft 서명이 유효하지 않은 파일은 지우고 exit 1. 이 폴더는 커밋하지 않는다(.gitignore).
param([switch]$Force)
. (Join-Path $PSScriptRoot 'env.ps1')
$ProgressPreference = 'SilentlyContinue'   # 진행 막대가 있으면 큰 파일 받기가 매우 느리다
$dir = Join-Path $Atmos.App 'windows\redist'
$items = @(
  @{ Name = 'VC_redist.x64.exe'; Url = 'https://aka.ms/vc14/vc_redist.x64.exe' },
  @{ Name = 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe'; Url = 'https://go.microsoft.com/fwlink/?linkid=2124701' }
)
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("fetched=$(Get-Date -Format o)")
$fail = 0
foreach ($i in $items) {
  $path = Join-Path $dir $i.Name
  if ($Force -or -not (Test-Path $path)) {
    Write-Output "GET  $($i.Url)"
    try { Invoke-WebRequest -Uri $i.Url -OutFile $path -UseBasicParsing }
    catch { Write-Output "FAIL $($i.Name) download: $($_.Exception.Message)"; $fail++; continue }
  }
  $sig = Get-AuthenticodeSignature -FilePath $path
  $signer = if ($sig.SignerCertificate) { $sig.SignerCertificate.Subject } else { '' }
  $ok = ($sig.Status -eq 'Valid') -and ($signer -match 'O=Microsoft Corporation')
  $line = '{0} {1}  {2} bytes  version={3}  sha256={4}  signature={5}  {6}' -f $(if ($ok) { 'OK  ' } else { 'FAIL' }), $i.Name,
    (Get-Item $path).Length, (Get-Item $path).VersionInfo.FileVersion, (Get-FileHash -Path $path -Algorithm SHA256).Hash, $sig.Status, $signer
  $lines.Add($line)
  Write-Output $line
  if (-not $ok) { Remove-Item -Path $path -Force; $fail++ }
}
$lines | Set-Content -Path (Join-Path $dir 'redist.txt') -Encoding UTF8
if ($fail -gt 0) { exit 1 }
exit 0
