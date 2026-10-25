# 설치 폴더 Legal의 PDF(The Awesome EULA, Third Party URLs and Logos)를 만든다(2026-10-25, Ableton Live의
# Ableton_Live_EULA_Third_Party_URL_Logo.pdf처럼: 표지·목차, EULA, 맨 뒤 제3자 URL·로고).
# -EulaDocx를 주면 그 Word 파일에서 EULA 원문을 글자 그대로 뽑아 windows\legal\eula_bilingual.txt를 먼저 바꾼다.
# 그 원문을 windows\legal\legal_document_template.html의 {{EULA}} 자리에 넣고 Edge를 화면 없이(headless) 임시
# 프로필로 띄워 인쇄한다(사용자 Edge 프로필은 건드리지 않는다). EULA나 틀이 바뀌었을 때만 돌리고,
# eula_bilingual.txt와 결과 PDF를 커밋한다.
param(
  [string]$EulaDocx = '',
  [string]$LegalDir = (Join-Path $PSScriptRoot '..\..\windows\legal'),
  [string]$PdfName = 'The_Awesome_EULA_Third_Party_URL_Logo.pdf'
)
$ErrorActionPreference = 'Stop'
$dir = (Resolve-Path $LegalDir).Path
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
$sb = New-Object Text.StringBuilder
$i = 0
$title = @()
while ($i -lt $lines.Count -and -not $lines[$i].StartsWith('[tbl] ')) { $title += $lines[$i]; $i++ }
[void]$sb.AppendLine('<h1>' + (($title | ForEach-Object { "<span$(Lang $_)>$(Esc $_)</span>" }) -join '<br>') + '</h1>')
if ($i -lt $lines.Count) {
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
$build = Join-Path $dir '_legal_document.build.html'
[IO.File]::WriteAllText($build, $template.Replace('{{EULA}}', $sb.ToString()), $utf8)

$edge = @(
  (Join-Path ${env:ProgramFiles(x86)} 'Microsoft\Edge\Application\msedge.exe'),
  (Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { Remove-Item $build; Write-Output 'FAIL Edge(msedge.exe)를 찾지 못했다'; exit 1 }
$out = Join-Path $dir $PdfName
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
