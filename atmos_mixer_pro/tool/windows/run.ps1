# Atmos Windows 하네스: 명령 하나를 돌리고 출력 전체를 증거 로그로 남긴다.
# 사용: powershell -NoProfile -ExecutionPolicy Bypass -File <이 파일> -Gate G4-03_cargo_check -Cwd D:\dev\Atmos\atmos_mixer_pro\rust -Run "cargo check"
# -Run의 명령줄은 cmd.exe에 그대로 넘긴다(PowerShell이 `--` 같은 인자를 바꾸지 않게). 큰따옴표가 필요한 명령은 이 스크립트 없이 직접 돌린다.
# 로그: D:\dev\logs\<yyyy-MM-dd>\<Gate>_<HHmmss>.log, 종료 코드: 명령의 종료 코드.
param(
  [string]$Gate = 'adhoc',
  [string]$Cwd = '',
  [string]$Run = ''
)
. (Join-Path $PSScriptRoot 'env.ps1')
if (-not $Run) { Write-Output 'usage: run.ps1 -Gate <id> [-Cwd <dir>] -Run "<command line>"'; exit 2 }
if (-not $Cwd) { $Cwd = $Atmos.App }
$dir = Join-Path $Atmos.Logs (Get-Date -Format 'yyyy-MM-dd')
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$log = Join-Path $dir ('{0}_{1}.log' -f $Gate, (Get-Date -Format 'HHmmss'))
$sw = New-Object System.IO.StreamWriter($log, $false, (New-Object System.Text.UTF8Encoding($false)))
$sw.AutoFlush = $true
$sw.WriteLine("# gate=$Gate")
$sw.WriteLine("# cwd=$Cwd")
$sw.WriteLine("# run=$Run")
$sw.WriteLine("# start=$(Get-Date -Format o)")
$code = 1
Push-Location $Cwd
try {
  & cmd.exe /d /s /c "chcp 65001>nul & $Run" 2>&1 | ForEach-Object {
    $line = "$_"
    $sw.WriteLine($line)
    $line
  }
  $code = $LASTEXITCODE
} catch {
  $sw.WriteLine("# exception: $_")
  Write-Output "exception: $_"
} finally {
  Pop-Location
  $sw.WriteLine("# exit=$code end=$(Get-Date -Format o)")
  $sw.Close()
}
Write-Output "LOG=$log EXIT=$code"
exit $code
