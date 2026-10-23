# Registers (or removes) the logon task that starts The Awesome's supervisor. The installer runs it
# (installer.iss). It replaces the HKCU Run value: on the Windows 11 dev PC (2026-10-23) Windows started
# every other Run entry at sign-in after a restart but skipped this one, twice, with no event or error.
# Usage: autostart.ps1              register for the current user (supervisor next to this script)
#        autostart.ps1 -Exe <path>  register another supervisor (testing)
#        autostart.ps1 -Remove      remove the task
param([string]$Exe = '', [switch]$Remove)

$ErrorActionPreference = 'Stop'
$name = 'The Awesome Autostart'

if ($Remove) {
  Unregister-ScheduledTask -TaskName $name -Confirm:$false -ErrorAction SilentlyContinue
  exit 0
}

if (-not $Exe) { $Exe = Join-Path $PSScriptRoot 'atmos_supervisor.exe' }
if (-not (Test-Path -LiteralPath $Exe)) { Write-Error "supervisor not found: $Exe"; exit 1 }
$user = [Security.Principal.WindowsIdentity]::GetCurrent().Name

$action = New-ScheduledTaskAction -Execute $Exe -Argument '--logon' -WorkingDirectory (Split-Path -Parent $Exe)
# A short delay lets the desktop and the USB audio driver settle. The app itself waits for the
# interface (30 s, then failover until it appears).
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $user
$trigger.Delay = 'PT10S'
# No time limit: the default (3 days) would end the supervisor during a long exhibition.
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -AllowStartIfOnBatteries `
  -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited
Register-ScheduledTask -TaskName $name -Action $action -Trigger $trigger -Settings $settings `
  -Principal $principal -Description 'Starts The Awesome at sign-in.' -Force | Out-Null
Write-Output "registered '$name' for $user -> $Exe --logon"
