# Atmos Windows 개발 PC 환경. 점 소싱해서 쓴다:  . D:\dev\Atmos\atmos_mixer_pro\tool\windows\env.ps1
# setup-env.ps1이 사용자 환경변수로 저장한 값을 지금 셸에도 바로 적용하고, 하네스 경로를 $Atmos에 담는다.
# D:가 아닌 PC는 점 소싱 전에 $AtmosDevRoot를 정한다(예: $AtmosDevRoot = 'E:\dev').
if (-not $AtmosDevRoot) { $AtmosDevRoot = 'D:\dev' }
$env:RUSTUP_HOME   = Join-Path $AtmosDevRoot 'rust\rustup'
$env:CARGO_HOME    = Join-Path $AtmosDevRoot 'rust\cargo'
$env:PUB_CACHE     = Join-Path $AtmosDevRoot 'pub-cache'
$env:LIBCLANG_PATH = Join-Path $AtmosDevRoot 'LLVM\bin'
$env:CPAL_ASIO_DIR = Join-Path $AtmosDevRoot 'sdk\asiosdk'
# Visual Studio 개발자 환경(VCINSTALLDIR·INCLUDE·LIB·PATH)을 이 셸에 넣는다. asio-sys 0.2.6 build.rs는 VCINSTALLDIR이 없으면
# vcvarsall.bat을 C:\Program Files 아래에서만 찾아, VS를 D:에 둔 PC에서는 "Could not find vcvarsall.bat"으로 빌드가 멈춘다.
# asio-sys가 하는 것과 같이 vcvarsall.bat amd64를 부르고 그 환경을 가져온다(이미 들어 있으면 건너뛴다).
if (-not $env:VCINSTALLDIR) {
  $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
  $vsPath = $null
  if (Test-Path $vswhere) {
    $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
  }
  if ($vsPath) {
    $vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvarsall.bat'
    if (Test-Path $vcvars) {
      # cmd /u로 set 결과를 UTF-16으로 받는다(한글 사용자 폴더가 든 TEMP·PATH가 코드 페이지 차이로 깨지지 않게).
      $psi = New-Object System.Diagnostics.ProcessStartInfo
      $psi.FileName = $env:ComSpec
      $psi.Arguments = '/d /u /c "call "' + $vcvars + '" amd64 >nul 2>&1 && set"'
      $psi.UseShellExecute = $false
      $psi.RedirectStandardOutput = $true
      $psi.StandardOutputEncoding = [System.Text.Encoding]::Unicode
      $psi.CreateNoWindow = $true
      $proc = [System.Diagnostics.Process]::Start($psi)
      $vsEnv = $proc.StandardOutput.ReadToEnd()
      $proc.WaitForExit()
      foreach ($line in ($vsEnv -split "`r?`n")) {
        $i = $line.IndexOf('=')
        if ($i -gt 0) { [Environment]::SetEnvironmentVariable($line.Substring(0, $i), $line.Substring($i + 1), 'Process') }
      }
    }
  }
}
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
