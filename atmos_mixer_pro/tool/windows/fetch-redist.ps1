# Atmos Windows 하네스 W4: 설치 파일에 넣을 Microsoft 재배포 파일을 받는다.
#   VC_redist.x64.exe                              Visual C++ 런타임(최신 지원판, aka.ms/vc14)
#   MicrosoftEdgeWebView2RuntimeInstallerX64.exe   WebView2 런타임 Evergreen Standalone(오프라인 설치용)
#   WebView2Loader_x86.dll                         WebView2 SDK(NuGet Microsoft.Web.WebView2, 앱과 같은 1.0.1210.39)의
#                                                  x86 로더. 32비트 설치 프로그램이 쓸 수 있는 런타임이 있는지 확인할 때 부른다
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> [-Force]
# 결과: atmos_mixer_pro\windows\redist\ 에 세 파일과 redist.txt(크기·버전·SHA256·서명). 있는 파일은 -Force 없이는 다시 받지 않는다.
#       Microsoft 서명이 유효하지 않은 파일은 지우고 exit 1. 이 폴더는 커밋하지 않는다(.gitignore).
param([switch]$Force)
. (Join-Path $PSScriptRoot 'env.ps1')
$ProgressPreference = 'SilentlyContinue'   # 진행 막대가 있으면 큰 파일 받기가 매우 느리다
$dir = Join-Path $Atmos.App 'windows\redist'
$items = @(
  @{ Name = 'VC_redist.x64.exe'; Url = 'https://aka.ms/vc14/vc_redist.x64.exe' },
  @{ Name = 'MicrosoftEdgeWebView2RuntimeInstallerX64.exe'; Url = 'https://go.microsoft.com/fwlink/?linkid=2124701' },
  # 패키지(zip)에서 Extract 한 파일만 꺼낸다.
  @{ Name = 'WebView2Loader_x86.dll'; Url = 'https://www.nuget.org/api/v2/package/Microsoft.Web.WebView2/1.0.1210.39';
     Extract = 'build/native/x86/WebView2Loader.dll' }
)
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$lines = New-Object System.Collections.Generic.List[string]
$lines.Add("fetched=$(Get-Date -Format o)")
$fail = 0
foreach ($i in $items) {
  $path = Join-Path $dir $i.Name
  if ($Force -or -not (Test-Path $path)) {
    Write-Output "GET  $($i.Url)"
    try {
      if ($i.Extract) {
        $pkg = Join-Path $env:TEMP ('atmos-redist-{0}.zip' -f [guid]::NewGuid())
        Invoke-WebRequest -Uri $i.Url -OutFile $pkg -UseBasicParsing
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [System.IO.Compression.ZipFile]::OpenRead($pkg)
        try {
          $entry = $zip.Entries | Where-Object { $_.FullName -eq $i.Extract } | Select-Object -First 1
          if (-not $entry) { throw "$($i.Extract) is not in the package" }
          [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $path, $true)
        } finally { $zip.Dispose(); Remove-Item -LiteralPath $pkg -Force -ErrorAction SilentlyContinue }
      } else { Invoke-WebRequest -Uri $i.Url -OutFile $path -UseBasicParsing }
    }
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
