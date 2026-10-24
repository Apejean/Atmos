# Registers (or removes) the logon task that starts The Awesome's supervisor. The installer runs it
# (installer.iss). It replaces the HKCU Run value: on the Windows 11 dev PC (2026-10-23) Windows started
# every other Run entry at sign-in after a restart but skipped this one, twice, with no event or error.
# The task is for the user signed in to this Windows session, read from the session, not for the account
# running the script: Setup runs it elevated, and with another administrator's UAC credentials that is the
# other administrator while the session still belongs to the operator (2026-10-24).
# Usage: autostart.ps1              register for the session user (supervisor next to this script)
#        autostart.ps1 -Exe <path>  register another supervisor (testing)
#        autostart.ps1 -Remove      remove the task
# Exit codes: 1 supervisor missing, 2 nobody signed in to this session, 3 the registered task is not the
# session user's or not limited/interactive.
param([string]$Exe = '', [switch]$Remove)

$ErrorActionPreference = 'Stop'
$name = 'The Awesome Autostart'

if ($Remove) {
  Unregister-ScheduledTask -TaskName $name -Confirm:$false -ErrorAction SilentlyContinue
  exit 0
}

if (-not $Exe) { $Exe = Join-Path $PSScriptRoot 'atmos_supervisor.exe' }
if (-not (Test-Path -LiteralPath $Exe)) { Write-Error "supervisor not found: $Exe"; exit 1 }

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class AwesomeSessionUser {
  [DllImport("wtsapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  static extern bool WTSQuerySessionInformation(IntPtr server, int sessionId, int infoClass, out IntPtr buffer, out int bytes);
  [DllImport("wtsapi32.dll")]
  static extern void WTSFreeMemory(IntPtr memory);
  // WTS_CURRENT_SERVER_HANDLE = 0, WTS_CURRENT_SESSION = -1
  static string Query(int infoClass) {
    IntPtr buffer; int bytes;
    if (!WTSQuerySessionInformation(IntPtr.Zero, -1, infoClass, out buffer, out bytes)) return null;
    try { return Marshal.PtrToStringUni(buffer); } finally { WTSFreeMemory(buffer); }
  }
  // WTSUserName = 5, WTSDomainName = 7
  public static string Name() {
    string user = Query(5), domain = Query(7);
    if (string.IsNullOrEmpty(user)) return null;
    return string.IsNullOrEmpty(domain) ? user : domain + "\\" + user;
  }
}
'@
function Get-Sid([string]$account) {
  (New-Object Security.Principal.NTAccount($account)).Translate([Security.Principal.SecurityIdentifier]).Value
}
$user = [AwesomeSessionUser]::Name()
if (-not $user) {
  Write-Error -ErrorAction Continue 'nobody is signed in to this Windows session; the logon task was not registered'
  exit 2
}
$sid = Get-Sid $user
$runner = [Security.Principal.WindowsIdentity]::GetCurrent().Name

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

# Read the task back: it must belong to the session user's SID and run with standard rights at sign-in.
$task = Get-ScheduledTask -TaskName $name
$taskSid = Get-Sid $task.Principal.UserId
$triggerSid = Get-Sid $task.Triggers[0].UserId
$summary = "session user $user ($sid), setup account $runner; task '$name' for $($task.Principal.UserId) " +
  "($taskSid), trigger $($task.Triggers[0].UserId), $($task.Principal.RunLevel), $($task.Principal.LogonType) -> $Exe --logon"
if ($taskSid -ne $sid -or $triggerSid -ne $sid -or "$($task.Principal.RunLevel)" -ne 'Limited' -or
    "$($task.Principal.LogonType)" -ne 'Interactive') {
  Write-Error -ErrorAction Continue "the registered task does not match: $summary"
  exit 3
}
Write-Output "registered: $summary"
