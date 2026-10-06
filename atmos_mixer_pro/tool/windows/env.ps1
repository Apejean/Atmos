# Atmos Windows 개발 PC 환경. 점 소싱해서 쓴다:  . D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1
# setup-env.ps1이 사용자 환경변수로 저장한 값을 지금 셸에도 바로 적용하고, 하네스 경로를 $Atmos에 담는다.
# D:가 아닌 PC는 점 소싱 전에 $AtmosDevRoot를 정한다(예: $AtmosDevRoot = 'E:\dev').
if (-not $AtmosDevRoot) { $AtmosDevRoot = 'D:\dev' }
$env:RUSTUP_HOME   = Join-Path $AtmosDevRoot 'rust\rustup'
$env:CARGO_HOME    = Join-Path $AtmosDevRoot 'rust\cargo'
$env:PUB_CACHE     = Join-Path $AtmosDevRoot 'pub-cache'
$env:LIBCLANG_PATH = Join-Path $AtmosDevRoot 'LLVM\bin'
$env:CPAL_ASIO_DIR = Join-Path $AtmosDevRoot 'sdk\asiosdk'
foreach ($rel in @('Git\cmd', 'InnoSetup6', 'flutter\bin', 'rust\cargo\bin')) {
  $p = Join-Path $AtmosDevRoot $rel
  if ((Test-Path $p) -and (($env:Path -split ';') -notcontains $p)) { $env:Path = "$p;$env:Path" }
}
# 한글 테스트 이름·로그가 깨지지 않게 네이티브 프로그램 출력을 UTF-8로 읽는다.
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$OutputEncoding = [Console]::OutputEncoding
# Windows PowerShell 5.1의 내려받기: TLS 1.2를 켜고 진행 표시줄을 끈다(표시줄이 내려받기를 크게 늦춘다).
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$ProgressPreference = 'SilentlyContinue'
$Atmos = @{
  Root      = $AtmosDevRoot
  Repo      = (Join-Path $AtmosDevRoot 'Atmos')
  App       = (Join-Path $AtmosDevRoot 'Atmos\atmos_mixer_pro')
  Tools     = (Join-Path $AtmosDevRoot 'Atmos\atmos_mixer_pro\tool\windows')
  Harness   = (Join-Path $AtmosDevRoot 'harness')
  Logs      = (Join-Path $AtmosDevRoot 'logs')
  Artifacts = (Join-Path $AtmosDevRoot 'artifacts')
  Backups   = (Join-Path $AtmosDevRoot 'backups')
  AppData   = (Join-Path $env:APPDATA 'com.example\atmos_mixer_pro')
  AppLogs   = (Join-Path $env:TEMP 'atmos_mixer_pro_logs')
}
