# 설치 폴더 Legal의 PDF(The Awesome EULA, Third Party URLs and Logos)를 만든다(2026-10-25, Ableton Live의
# Ableton_Live_EULA_Third_Party_URL_Logo.pdf처럼: 표지·목차, EULA, 맨 뒤 제3자 URL·로고).
# -EulaDocx를 주면 그 Word 파일에서 EULA 원문을 글자 그대로 뽑아 windows\legal\eula_bilingual.txt를 먼저 바꾼다.
# 그 원문을 windows\legal\legal_document_template.html의 {{EULA}} 자리에 넣고 Edge를 화면 없이(headless) 임시
# 프로필로 띄워 인쇄한다(사용자 Edge 프로필은 건드리지 않는다). EULA나 틀이 바뀌었을 때만 돌리고,
# eula_bilingual.txt와 결과 PDF를 커밋한다.
# -Platform macos(2026-11-28, Mac 세션 요청): macOS 앱 Legal에 넣을 판을 macos\legal\The_Awesome_EULA.pdf로 만든다. 같은
# 원문에서 Windows에만 해당하는 것(제7조의 Steinberg ASIO® 문구와 Microsoft 구성요소 문단, 틀의 <!--windows--> 구역 =
# 제3자 URL·ASIO 로고 쪽)을 뺀다. 원문이 바뀌어 뺄 문장을 찾지 못하면 멈춘다(빠뜨린 채 만들지 않는다).
param(
  [string]$EulaDocx = '',
  [string]$LegalDir = (Join-Path $PSScriptRoot '..\..\windows\legal'),
  [ValidateSet('windows', 'macos')][string]$Platform = 'windows',
  [string]$OutDir = '',
  [string]$PdfName = ''
)
$ErrorActionPreference = 'Stop'
$dir = (Resolve-Path $LegalDir).Path
if (-not $OutDir) { $OutDir = if ($Platform -eq 'macos') { Join-Path $PSScriptRoot '..\..\macos\legal' } else { $dir } }
if (-not $PdfName) { $PdfName = if ($Platform -eq 'macos') { 'The_Awesome_EULA.pdf' } else { 'The_Awesome_EULA_Third_Party_URL_Logo.pdf' } }
$eulaTxt = Join-Path $dir 'eula_bilingual.txt'
$utf8 = New-Object Text.UTF8Encoding($false)

if ($EulaDocx) {
  # Word 문단을 한 줄씩(표 안 문단은 "[tbl] "을 붙여) 글자 그대로 옮긴다.
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path $EulaDocx).Path)
  try {
    $sr = New-Object IO.StreamReader($zip.GetEntry('word/document.xml').Open(), [Text.Encoding]::UTF8)
    $xml = [xml]$sr.ReadToEnd()
    $sr.Close()
  } finally { $zip.Dispose() }
  $ns = New-Object Xml.XmlNamespaceManager($xml.NameTable)
  $ns.AddNamespace('w', 'http://schemas.openxmlformats.org/wordprocessingml/2006/main')
  $paras = foreach ($p in $xml.SelectNodes('//w:body//w:p', $ns)) {
    # 문단 안 줄바꿈(w:br, w:cr)은 따로 한 줄로 옮긴다(예: 영어·한국어 저작권 줄).
    $text = ($p.SelectNodes('.//w:t|.//w:tab|.//w:br|.//w:cr', $ns) | ForEach-Object {
        $node = $_
        switch ($node.LocalName) { 't' { $node.InnerText } 'tab' { ' ' } default { "`n" } }
      }) -join ''
    $prefix = if ($p.SelectSingleNode('ancestor::w:tbl', $ns)) { '[tbl] ' } else { '' }
    $text -split "`n" | ForEach-Object { "$prefix$_" }
  }
  [IO.File]::WriteAllLines($eulaTxt, [string[]]$paras, $utf8)
  Write-Output "EULA text: $eulaTxt ($($paras.Count) paragraphs)"
}

# EULA 원문을 HTML로: 제목(첫 표 앞), 검토용 표시(표), 판·시행일·저작권, 조항 제목(영어 "N. "과 한국어 "제N조. "),
# ENGLISH/한국어 머리, (a) 항목, 문단. 글자는 바꾸지 않고 &·<·>만 바꾼다.
function Esc([string]$s) { $s.Replace('&', '&amp;').Replace('<', '&lt;').Replace('>', '&gt;') }
function Lang([string]$s) { if ($s -match '[가-힣]') { ' lang="ko"' } else { '' } }
$lines = @([IO.File]::ReadAllLines($eulaTxt, [Text.Encoding]::UTF8) | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
if ($Platform -eq 'macos') {
  # 제7조: ASIO는 Windows 전용(ASIO 계약 3조 1항 g — macOS 판에 ASIO 문구·로고를 두지 않는다), Microsoft 구성요소는 Windows 설치만.
  $cut = @(
    @('Third-party components, including Steinberg ASIO® technology where applicable, remain subject', 'Third-party components remain subject'),
    @('해당하는 경우 Steinberg ASIO® 기술을 포함한 제3자 구성요소에는', '제3자 구성요소에는')
  )
  foreach ($c in $cut) {
    $hit = @($lines | Where-Object { $_.Contains($c[0]) }).Count
    if ($hit -ne 1) { Write-Output "FAIL macOS 판: 바꿀 문장을 $hit 곳에서 찾음: $($c[0])"; exit 1 }
    $lines = @($lines | ForEach-Object { $_.Replace($c[0], $c[1]) })
  }
  foreach ($start in @('On Windows, the Software installs and uses the Microsoft', 'Windows에서 프로그램은 Microsoft')) {
    $hit = @($lines | Where-Object { $_.StartsWith($start) }).Count
    if ($hit -ne 1) { Write-Output "FAIL macOS 판: 뺄 문단을 $hit 곳에서 찾음: $start"; exit 1 }
    $lines = @($lines | Where-Object { -not $_.StartsWith($start) })
  }
  $left = @($lines | Where-Object { $_ -cmatch 'ASIO|Microsoft|WebView2' })
  if ($left.Count) { Write-Output "FAIL macOS 판에 Windows 문장이 남음: $($left -join ' / ')"; exit 1 }
}
$sb = New-Object Text.StringBuilder
$i = 0
$title = @()
# 확정본(2026-11-28)에는 검토용 표시 표가 없으므로 제목은 "Version" 줄 앞에서도 끝난다.
while ($i -lt $lines.Count -and -not $lines[$i].StartsWith('[tbl] ') -and $lines[$i] -notmatch '^Version\s') { $title += $lines[$i]; $i++ }
[void]$sb.AppendLine('<h1>' + (($title | ForEach-Object { "<span$(Lang $_)>$(Esc $_)</span>" }) -join '<br>') + '</h1>')
if ($i -lt $lines.Count -and $lines[$i].StartsWith('[tbl] ')) {
  [void]$sb.AppendLine('<div class="draft">')
  while ($i -lt $lines.Count -and $lines[$i].StartsWith('[tbl] ')) {
    $t = $lines[$i].Substring(6); [void]$sb.AppendLine("<p$(Lang $t)>$(Esc $t)</p>"); $i++
  }
  [void]$sb.AppendLine('</div>')
}
while ($i -lt $lines.Count -and $lines[$i] -notmatch '^\d+\.\s') {
  [void]$sb.AppendLine("<p class=`"meta`"$(Lang $lines[$i])>$(Esc $lines[$i])</p>"); $i++
}
# 조항 제목과 ENGLISH/한국어 머리는 첫 문단과 한 묶음(.keep)으로 둬 쪽 끝에 제목만 남지 않게 한다.
$keep = $false
for (; $i -lt $lines.Count; $i++) {
  $l = $lines[$i]
  if ($l -match '^\d+\.\s') {
    if ($keep) { [void]$sb.AppendLine('</div>') }
    $ko = ''
    if ($i + 1 -lt $lines.Count -and $lines[$i + 1] -match '^제\d+조\.') { $i++; $ko = "<span class=`"ko`" lang=`"ko`">$(Esc $lines[$i])</span>" }
    [void]$sb.AppendLine('<div class="keep">')
    [void]$sb.AppendLine("<h2>$(Esc $l)$ko</h2>")
    $keep = $true
  } elseif ($l -eq 'ENGLISH' -or $l -eq '한국어') {
    if (-not $keep) { [void]$sb.AppendLine('<div class="keep">'); $keep = $true }
    [void]$sb.AppendLine("<h3$(Lang $l)>$(Esc $l)</h3>")
  } else {
    $class = if ($l -match '^\([a-z]\)\s') { ' class="item"' } else { '' }
    [void]$sb.AppendLine("<p$class$(Lang $l)>$(Esc $l)</p>")
    if ($keep) { [void]$sb.AppendLine('</div>'); $keep = $false }
  }
}
if ($keep) { [void]$sb.AppendLine('</div>') }

$template = [IO.File]::ReadAllText((Join-Path $dir 'legal_document_template.html'), [Text.Encoding]::UTF8)
if (-not $template.Contains('{{EULA}}')) { Write-Output 'FAIL 틀에 {{EULA}} 자리가 없다'; exit 1 }
# <!--windows--> … <!--/windows--> 구역(제목 꼬리, 목차의 제3자 줄, 제3자 URL·ASIO 로고 쪽)은 Windows 판에만 둔다.
$template = if ($Platform -eq 'macos') { [regex]::Replace($template, '(?s)<!--windows-->.*?<!--/windows-->', '') } else { $template.Replace('<!--windows-->', '').Replace('<!--/windows-->', '') }
$build = Join-Path $dir '_legal_document.build.html'
[IO.File]::WriteAllText($build, $template.Replace('{{EULA}}', $sb.ToString()), $utf8)

$edge = @(
  (Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'),
  (Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { Remove-Item $build; Write-Output 'FAIL Edge(msedge.exe)를 찾지 못했다'; exit 1 }
New-Item -ItemType Directory -Force $OutDir | Out-Null
$out = Join-Path (Resolve-Path $OutDir).Path $PdfName
$profileDir = Join-Path $env:TEMP ('legal-pdf-' + [guid]::NewGuid().ToString('N'))
if (Test-Path $out) { Remove-Item $out }
$p = Start-Process -FilePath $edge -PassThru -Wait -ArgumentList @(
  '--headless', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
  "--user-data-dir=`"$profileDir`"", '--no-pdf-header-footer', "--print-to-pdf=`"$out`"", ('file:///' + $build.Replace('\', '/'))
)
for ($n = 0; $n -lt 20 -and -not (Test-Path $out); $n++) { Start-Sleep -Milliseconds 500 }
Remove-Item $profileDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $build -ErrorAction SilentlyContinue
if (-not (Test-Path $out)) { Write-Output "FAIL PDF가 만들어지지 않았다(exit $($p.ExitCode))"; exit 1 }
Write-Output ('OK {0} {1} bytes' -f $out, (Get-Item $out).Length)
