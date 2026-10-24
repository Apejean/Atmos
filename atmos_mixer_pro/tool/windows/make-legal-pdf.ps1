# 설치 폴더 Legal의 PDF(The Awesome Third Party URLs and Logos)를 HTML 원본에서 다시 만든다.
# Edge를 화면 없이(headless) 임시 프로필로 띄워 인쇄한다(사용자 Edge 프로필은 건드리지 않는다).
# 원본(windows\legal\third_party_urls_and_logos.html)이나 로고가 바뀌었을 때만 돌리고, 결과 PDF를 커밋한다.
param(
  [string]$Html = (Join-Path $PSScriptRoot '..\..\windows\legal\third_party_urls_and_logos.html'),
  [string]$Pdf = (Join-Path $PSScriptRoot '..\..\windows\legal\The_Awesome_Third_Party_URL_Logo.pdf')
)
$edge = @(
  (Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'),
  (Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { Write-Output 'FAIL Edge(msedge.exe)를 찾지 못했다'; exit 1 }

$src = (Resolve-Path $Html).Path
$out = [IO.Path]::GetFullPath($Pdf)
$profileDir = Join-Path $env:TEMP ('legal-pdf-' + [guid]::NewGuid().ToString('N'))
if (Test-Path $out) { Remove-Item $out }
$url = 'file:///' + $src.Replace('\', '/')
$p = Start-Process -FilePath $edge -PassThru -Wait -ArgumentList @(
  '--headless', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
  "--user-data-dir=`"$profileDir`"", '--no-pdf-header-footer', "--print-to-pdf=`"$out`"", $url
)
for ($i = 0; $i -lt 20 -and -not (Test-Path $out); $i++) { Start-Sleep -Milliseconds 500 }
Remove-Item $profileDir -Recurse -Force -ErrorAction SilentlyContinue
if (-not (Test-Path $out)) { Write-Output "FAIL PDF가 만들어지지 않았다(exit $($p.ExitCode))"; exit 1 }
Write-Output ('OK {0} {1} bytes' -f $out, (Get-Item $out).Length)
