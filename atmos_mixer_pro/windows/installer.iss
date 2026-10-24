; Folder with the built app (Release folder + supervisor exe). Default: the repository build, one level up
; from this script. For the CI zip: ISCC /DReleaseDir=<folder> windows\installer.iss
#ifndef ReleaseDir
  #define ReleaseDir AddBackslash(SourcePath) + "..\build\windows\x64\runner\Release"
#endif
; Product name shown in the program list, shortcuts and new install folder (2026-10-22: renamed from "Atmos Mixer Pro").
; An upgrade keeps the existing install folder (UsePreviousAppDir). The setup file name stays AtmosMixerPro_Setup
; because the release workflow renames it.
#define MyAppName "The Awesome"
#define MyAppPublisher "The Awesome"
#define MyAppExeName "atmos_mixer_pro.exe"
; The app sets the same ID on itself (runner\main.cpp), so the taskbar matches its window to the shortcuts: the
; right-click menu shows the shortcut name, not the exe description Windows cached, and pinning pins the shortcut.
#define MyAppUserModelID "TheAwesome.AtmosMixerPro"
; The supervisor relaunches the app after a crash or a hang. Shortcuts and the logon task (autostart.ps1) start the
; supervisor, which then starts the app. Setup does not start it at the end (2026-10-24): that step ran as the
; pre-UAC user through Setup's first window, and an antivirus sandbox on that window made it fail with
; "CallSpawnServer: Unexpected response".
#define SupervisorExeName "atmos_supervisor.exe"
; The version is read from the built exe. Flutter writes pubspec.yaml's version into the exe's
; ProductVersion, so the installer always matches the app. The path uses the same ReleaseDir as [Files].
#define MyAppExePath AddBackslash(ReleaseDir) + MyAppExeName
#define MyAppVersion GetStringFileInfo(MyAppExePath, "ProductVersion")
#if MyAppVersion == ""
  #error Could not read the version from the built exe. Run "flutter build windows" first or pass /DReleaseDir.
#endif
; Microsoft prerequisites bundled for offline installs: the Visual C++ runtime and the WebView2 runtime
; (3D room viewer). tool\windows\fetch-redist.ps1 downloads them into windows\redist (not committed).
; Each one is installed only when it is missing or older; see InstallPrerequisites in [Code].
#ifndef RedistDir
  #define RedistDir AddBackslash(SourcePath) + "redist"
#endif
; Legal folder (2026-10-25, like Ableton Live): the open-source license texts that
; tool\legal\generate_third_party_licenses.dart writes after the build, and the PDF with the end user license
; agreement and the third-party URLs and logos (windows\legal, printed by tool\windows\make-legal-pdf.ps1 from
; eula_bilingual.txt and legal_document_template.html). Setup is not built without them.
#ifndef LegalDir
  #define LegalDir AddBackslash(SourcePath) + "..\build\legal"
#endif
#define LegalLicensesTxt "third_party_license_information.txt"
#define LegalEulaPdf "The_Awesome_EULA_Third_Party_URL_Logo.pdf"
#if !FileExists(AddBackslash(LegalDir) + LegalLicensesTxt)
  #error Missing the third-party license file. Run tool\legal\generate_third_party_licenses.dart after the build or pass /DLegalDir.
#endif
#define VCRedistExe "VC_redist.x64.exe"
#define WebView2Exe "MicrosoftEdgeWebView2RuntimeInstallerX64.exe"
; x86 WebView2 loader (Setup is 32-bit) for the official check of a usable runtime, see UsableWebView2 in [Code].
#define WebView2LoaderX86 "WebView2Loader_x86.dll"
#if !FileExists(AddBackslash(RedistDir) + VCRedistExe) || !FileExists(AddBackslash(RedistDir) + WebView2Exe) || !FileExists(AddBackslash(RedistDir) + WebView2LoaderX86)
  #error Missing Microsoft redistributables. Run tool\windows\fetch-redist.ps1 first or pass /DRedistDir.
#endif
#define VCRedistVersion GetVersionNumbersString(AddBackslash(RedistDir) + VCRedistExe)
; The WebView2 installer's file version is Edge Update's (1.3.x), not the runtime's, so the runtime it carries is
; written here: 154.0.4258.62 in the one fetch-redist.ps1 got on 2026-10-08 (it always gets the latest). Keep it equal
; to the bundled runtime: a usable registered runtime at least this new is kept, and an unusable one of exactly this
; version is repaired with the bundled installer. Never set it above the bundled runtime: a registered runtime newer
; than the bundled one would then run the bundled installer, which fails on it. A test build can pass /DWebView2MinVersion=<v>.
#ifndef WebView2MinVersion
  #define WebView2MinVersion "154.0.4258.62"
#endif
; Free space (MB) wanted on the Program Files (x86) drive before the WebView2 runtime is installed or repaired: its
; folder is about 0.9 GB, the bundled installer is extracted to {tmp} (0.2 GB), and the older runtime folder stays
; until Edge Update removes it. Below this Setup asks before installing. A test build can pass /DWebView2MinFreeMB=<n>.
#ifndef WebView2MinFreeMB
  #define WebView2MinFreeMB 2048
#endif

[Setup]
AppId={{D37EAB59-7A77-4E2C-964B-FEFA2C3058D6}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DisableProgramGroupPage=yes
OutputBaseFilename=AtmosMixerPro_Setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
; %TEMP%\Setup Log <date> #<n>.txt: shows whether the prerequisites were installed or skipped.
SetupLogging=yes
; No Restart Manager check. On this PC under Windows 10 (2026-10-12) a first install died silently in RmGetList,
; with Inno 6.7.1 and 6.7.3 alike, while upgrades and Windows Sandbox passed; the field PC gets a first install too.
; PrepareToInstall stops the supervisor and the app itself before any file is copied, so the check finds nothing.
CloseApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
; First on purpose: with solid compression a file comes out only after everything before it, and Setup needs this one
; before installing (last in the list it took 13 s to extract).
Source: "{#RedistDir}\{#WebView2LoaderX86}"; Flags: dontcopy
Source: "{#ReleaseDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; The supervisor is copied next to the app by the build (CI copies it into the Release folder).
Source: "{#ReleaseDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; Registers the logon task that starts the supervisor ([Run] below).
Source: "autostart.ps1"; DestDir: "{app}"; Flags: ignoreversion
; Legal folder (see LegalDir above). Uninstall removes it with the other installed files.
Source: "{#LegalDir}\{#LegalLicensesTxt}"; DestDir: "{app}\Legal"; DestName: "Third-party License Information.txt"; Flags: ignoreversion
Source: "legal\{#LegalEulaPdf}"; DestDir: "{app}\Legal"; Flags: ignoreversion
; Extracted to {tmp} only when needed (InstallPrerequisites).
Source: "{#RedistDir}\{#VCRedistExe}"; Flags: dontcopy
Source: "{#RedistDir}\{#WebView2Exe}"; Flags: dontcopy

[InstallDelete]
; Shortcuts from before the rename. Without this an upgrade leaves "Atmos Mixer Pro" next to the new shortcuts.
Type: files; Name: "{autoprograms}\Atmos Mixer Pro.lnk"
Type: files; Name: "{autodesktop}\Atmos Mixer Pro.lnk"
; Assets that earlier versions installed and this one dropped. An upgrade only overwrites files, so they stayed
; behind (2026-10-24: the old listener bust models, whose source was unknown).
; Name each one: deleting the whole data\flutter_assets folder left an install without it on the dev PC, where
; an antivirus sandbox ran Setup two or three times (2026-10-24).
Type: files; Name: "{app}\data\flutter_assets\assets\models\listener_bust.obj"
Type: files; Name: "{app}\data\flutter_assets\assets\models\listener_bust_fixed.obj"
Type: files; Name: "{app}\data\flutter_assets\assets\models\listener_head.obj"
Type: files; Name: "{app}\data\flutter_assets\assets\models\listener_head_mannequin.obj"
Type: files; Name: "{app}\data\flutter_assets\assets\models\listener_head.glb"
Type: files; Name: "{app}\data\flutter_assets\assets\models\listener_mannequin.glb"
Type: files; Name: "{app}\data\flutter_assets\assets\models\room_with_listener.glb"
Type: files; Name: "{app}\data\flutter_assets\assets\models\room_temp.glb"
Type: files; Name: "{app}\data\flutter_assets\assets\models\scale_room.py"

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"; AppUserModelID: "{#MyAppUserModelID}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"; AppUserModelID: "{#MyAppUserModelID}"; Tasks: desktopicon

[Registry]
; Earlier versions started the supervisor from the HKCU Run key. Windows 11 skipped that entry at sign-in
; after a restart (2026-10-23, dev PC, every restart) while it started the other Run entries, so the logon
; task replaces it. Remove the old value on upgrade so the two never start together.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "AtmosMixerPro"; Flags: deletevalue
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"; ValueType: none; ValueName: "AtmosMixerPro"; Flags: deletevalue

[Run]
; OSC arrives over UDP on 0.0.0.0. Allow it here instead of the first-run firewall prompt, which nobody answers
; on an unattended PC: any sender on private and domain networks, only the local subnet on public networks.
; Rules for this exe that the prompt may have left (allow or block) are removed first.
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=all program=""{app}\{#MyAppExeName}"""; Flags: runhidden waituntilterminated
; Rules from before the rename, by name too. On the dev PC (2026-10-23) both deletes exited 1 and old rules stayed:
; the antivirus (Avast) first ran Setup in its sandbox, whose netsh changes reached the running firewall but not the
; stored rules, so the two lists disagreed (firewall events 2052/2097). Check the cleanup on a PC whose security
; software leaves Setup alone (WINDOWS_FIELD_CHECKLIST.md sections 0 and 5).
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=""Atmos Mixer Pro OSC"""; Flags: runhidden waituntilterminated
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=""Atmos Mixer Pro OSC (public, local subnet)"""; Flags: runhidden waituntilterminated
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall add rule name=""The Awesome OSC"" dir=in action=allow program=""{app}\{#MyAppExeName}"" protocol=UDP profile=private,domain enable=yes"; Flags: runhidden waituntilterminated
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall add rule name=""The Awesome OSC (public, local subnet)"" dir=in action=allow program=""{app}\{#MyAppExeName}"" protocol=UDP profile=public remoteip=localsubnet enable=yes"; Flags: runhidden waituntilterminated
; Logon task "The Awesome Autostart" for the signed-in user (not the elevated account): starts the supervisor with
; --logon 10 s after sign-in, with no run-time limit (the Task Scheduler default of 3 days would end a long show).
; It runs in the elevated Setup and finds that user from the Windows session, so it no longer needs Setup's first
; window (runasoriginaluser), which an antivirus sandbox broke on 2026-10-24. Its check line goes to the Setup log.
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\autostart.ps1"""; Flags: runhidden waituntilterminated logoutput

[UninstallRun]
; Stop the supervisor first. If the app is stopped first, the supervisor treats it as a crash and starts it again.
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#SupervisorExeName}"; Flags: runhidden; RunOnceId: "StopSupervisor"
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#MyAppExeName}"; Flags: runhidden; RunOnceId: "StopApp"
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=all program=""{app}\{#MyAppExeName}"""; Flags: runhidden; RunOnceId: "DeleteFirewallRules"
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /TN ""The Awesome Autostart"" /F"; Flags: runhidden; RunOnceId: "DeleteAutostartTask"

[Code]
// Installing over a running show (an upgrade): stop the supervisor before the app, for the same
// reason as at uninstall. Otherwise the supervisor starts the app again and its files stay locked.
procedure StopAtmos();
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#SupervisorExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#MyAppExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

const
  VCRuntimeKey = 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64';
  WebView2Key = 'Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  // WebView2Action
  WebView2Keep = 0;      // a usable runtime at least as new as the bundled one: left alone
  WebView2Install = 1;   // none registered, or older than the bundled one
  WebView2Repair = 2;    // the bundled version is registered but not usable (WebView2Problem)
  WebView2Unusable = 3;  // a newer version is registered but not usable: left alone, reported

var
  PrerequisiteNeedsRestart: Boolean;
  // Prerequisites still missing when Setup ends: 1 Visual C++, 2 WebView2 (see GetCustomSetupExitCode).
  PrerequisiteFailures: Integer;
  // Decided in PrepareToInstall (DecideWebView2), carried out in InstallPrerequisites.
  WebView2Action: Integer;
  WebView2Registered: String;

// Installed x64 Visual C++ runtime version in one registry view, 0 when missing.
function VCRuntimeVersion(RootKey: Integer): Int64;
var
  Installed, Major, Minor, Bld: Cardinal;
begin
  Result := 0;
  if RegQueryDWordValue(RootKey, VCRuntimeKey, 'Installed', Installed) and (Installed = 1) and
     RegQueryDWordValue(RootKey, VCRuntimeKey, 'Major', Major) and
     RegQueryDWordValue(RootKey, VCRuntimeKey, 'Minor', Minor) and
     RegQueryDWordValue(RootKey, VCRuntimeKey, 'Bld', Bld) then
    Result := PackVersionComponents(Major, Minor, Bld, 0);
end;

// Newer packages register in the 64-bit view and older ones in the 32-bit view, so take the newer.
// The runtime must be at least as new as the toolset that built the app (CI: MSVC 14.51); the bundled
// package is the latest supported one, so anything older than it is replaced.
function NeedsVCRuntime(): Boolean;
var
  Installed, Bundled: Int64;
begin
  Installed := VCRuntimeVersion(HKLM64);
  if ComparePackedVersion(VCRuntimeVersion(HKLM32), Installed) > 0 then
    Installed := VCRuntimeVersion(HKLM32);
  Result := not StrToVersion('{#VCRedistVersion}', Bundled) or (ComparePackedVersion(Installed, Bundled) < 0);
end;

// Keeps the newer of Newest and the WebView2 runtime registered ("pv") in one registry view. 0.0.0.0 and values that
// are not versions count as missing.
procedure TakeNewerWebView2(RootKey: Integer; const SubKey: String; var Newest: Int64; var NewestText: String);
var
  Version: String;
  Ver: Int64;
begin
  if RegQueryStringValue(RootKey, SubKey, 'pv', Version) and StrToVersion(Version, Ver) and
     (ComparePackedVersion(Ver, Newest) > 0) then begin
    Newest := Ver;
    NewestText := Version;
  end;
end;

// The official check (Microsoft, "Detect if a WebView2 Runtime is already installed", approach 2), the same call the
// app makes before it starts the 3D room viewer: is there a runtime the WebView2 loader can use? Setup is 32-bit, so it
// calls the x86 loader of the WebView2 SDK the app uses (1.0.1210.39). delayload: when the loader cannot be loaded, the
// call raises and the check counts as unknown.
function GetAvailableCoreWebView2BrowserVersionString(BrowserExecutableFolder: Cardinal; var VersionInfo: Cardinal): Integer;
  external 'GetAvailableCoreWebView2BrowserVersionString@files:{#WebView2LoaderX86} stdcall delayload';
procedure CoTaskMemFree(Block: Cardinal);
  external 'CoTaskMemFree@ole32.dll stdcall';
function lstrlenW(Str: Cardinal): Integer;
  external 'lstrlenW@kernel32.dll stdcall';
function lstrcpynW(Dest: String; Src: Cardinal; MaxLength: Integer): Cardinal;
  external 'lstrcpynW@kernel32.dll stdcall';

// Version of the runtime the loader would use, '' when it finds none. Known is False when the loader could not be called.
function UsableWebView2(var Known: Boolean): String;
var
  Hr: Integer;
  Info: Cardinal;
begin
  Result := '';
  Known := False;
  Info := 0;
  try
    Hr := GetAvailableCoreWebView2BrowserVersionString(0, Info);
    Known := True;
  except
    Log('WebView2 loader check could not run: ' + GetExceptionMessage);
    Exit;
  end;
  if (Hr = 0) and (Info <> 0) then begin
    Result := StringOfChar(' ', lstrlenW(Info));
    lstrcpynW(Result, Info, Length(Result) + 1);
  end else
    Log(Format('WebView2 loader finds no usable runtime (HRESULT %d = 0x%.8x)', [Hr, Hr]));
  if Info <> 0 then
    CoTaskMemFree(Info);
end;

// Runtime file missing in Folder, '' when none is: msedgewebview2.exe, and for the bundled version, whose folder layout
// is known (Z6), also the x64 EmbeddedBrowserWebView.dll the 64-bit app loads (renamed away, the loader still named 154
// and the bundled installer put it back). Newer versions get only the first check, so a changed layout cannot make a
// healthy newer runtime look broken.
function MissingIn(const Folder, Version: String): String;
begin
  Result := '';
  if not FileExists(Folder + '\msedgewebview2.exe') then
    Result := 'msedgewebview2.exe'
  else if (Version = '{#WebView2MinVersion}') and not FileExists(Folder + '\EBWebView\x64\EmbeddedBrowserWebView.dll') then
    Result := 'EBWebView\x64\EmbeddedBrowserWebView.dll';
end;

// The loader check does not look at the runtime files: with msedgewebview2.exe renamed away it still named 154 (Z6). So
// also look in the folder Edge Update keeps for the version the loader names ("location" next to "pv"), see MissingIn.
// Not a check Microsoft documents: when no location is registered nothing counts as missing, so a healthy runtime is
// never reinstalled because of it. Returns the missing file, '' when none is.
function MissingWebView2File(Version: String): String;
var
  Location, Missing: String;
begin
  Result := '';
  if Pos(' ', Version) > 0 then
    Version := Copy(Version, 1, Pos(' ', Version) - 1);
  if RegQueryStringValue(HKLM32, 'SOFTWARE\' + WebView2Key, 'location', Location) then begin
    Result := MissingIn(AddBackslash(Location) + Version, Version);
    if Result = '' then
      Exit;
  end;
  if RegQueryStringValue(HKCU, 'Software\' + WebView2Key, 'location', Location) then begin
    Missing := MissingIn(AddBackslash(Location) + Version, Version);
    if (Missing = '') or (Result = '') then
      Result := Missing;
  end;
end;

// '' when a usable runtime is found (or the loader cannot be asked), else what is wrong. Version: what the loader uses.
function WebView2Problem(var Version: String): String;
var
  Known: Boolean;
begin
  Result := '';
  Version := UsableWebView2(Known);
  if not Known then
    Version := '(loader check unavailable)'
  else if Version = '' then
    Result := 'the WebView2 loader finds no usable runtime'
  else begin
    Result := MissingWebView2File(Version);
    if Result <> '' then
      Result := Result + ' of ' + Version + ' is missing';
  end;
end;

// What to do with WebView2 (WebView2Action), from the newest registered runtime ("pv", per machine or per user) and the
// checks above. Registered alone was not enough: Windows Sandbox (Z6, 2026-10-11) registered a 141 runtime with its files
// that the loader could not use, so the 3D room viewer did not start. A usable runtime at least as new as the bundled
// one is never reinstalled. The bundled installer repairs its own version (it put a missing msedgewebview2.exe back in
// Z6), but over a newer registration it fails (-2147219187 = 0x8004090D in Z6), so a broken newer runtime is left alone
// and reported.
procedure DecideWebView2();
var
  Newest, MinVersion: Int64;
  Usable, Problem: String;
begin
  Newest := 0;
  WebView2Registered := 'none';
  TakeNewerWebView2(HKLM32, 'SOFTWARE\' + WebView2Key, Newest, WebView2Registered);
  TakeNewerWebView2(HKCU, 'Software\' + WebView2Key, Newest, WebView2Registered);
  if not StrToVersion('{#WebView2MinVersion}', MinVersion) or (ComparePackedVersion(Newest, MinVersion) < 0) then begin
    WebView2Action := WebView2Install;
    Log('WebView2 Runtime registered: ' + WebView2Registered + ' (older than {#WebView2MinVersion} or none): install');
    Exit;
  end;
  Problem := WebView2Problem(Usable);
  if Problem = '' then begin
    WebView2Action := WebView2Keep;
    Log('WebView2 Runtime registered: ' + WebView2Registered + ', loader uses ' + Usable + ': keep');
  end else if ComparePackedVersion(Newest, MinVersion) = 0 then begin
    WebView2Action := WebView2Repair;
    Log('WebView2 Runtime registered: ' + WebView2Registered + ', ' + Problem + ': repair with the bundled installer');
  end else begin
    WebView2Action := WebView2Unusable;
    Log('WebView2 Runtime registered: ' + WebView2Registered + ', ' + Problem + ', newer than {#WebView2MinVersion}: left alone');
  end;
end;

// Drive of Program Files (x86), where the WebView2 runtime goes.
function WebView2Drive(): String;
begin
  Result := ExtractFileDrive(ExpandConstant('{commonpf32}'));
end;

// A full disk was one cause of a failed prerequisite (Z6: -2147219198 = 0x80040902).
function FreeSpaceHint(): String;
begin
  Result := 'If drive ' + WebView2Drive() + ' is nearly full, free up space and run this setup again.';
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Free, Total: Int64;
begin
  Result := '';
  DecideWebView2();
  if ((WebView2Action = WebView2Install) or (WebView2Action = WebView2Repair)) and
     GetSpaceOnDisk64(WebView2Drive() + '\', Free, Total) and (Free div 1048576 < {#WebView2MinFreeMB}) then begin
    Log('Free space on drive ' + WebView2Drive() + ' ' + IntToStr(Free div 1048576) + ' MB, the WebView2 Runtime wants {#WebView2MinFreeMB} MB');
    if SuppressibleMsgBox('Drive ' + WebView2Drive() + ' has ' + IntToStr(Free div 1048576) + ' MB free. Installing the ' +
         'Microsoft Edge WebView2 Runtime (3D room viewer) needs about {#WebView2MinFreeMB} MB and can fail without it.' + #13#10#13#10 +
         'Yes: install anyway. No: stop, free up space and run this setup again.',
         mbConfirmation, MB_YESNO, IDYES) = IDNO then begin
      Result := 'Stopped to free up space on drive ' + WebView2Drive() + '. Run this setup again afterwards.';
      Exit;
    end;
  end;
  StopAtmos();
end;

// Runs a bundled prerequisite from {tmp}. ResultCode is -1 when it could not be started.
procedure RunPrerequisite(const FileName, Params: String; var ResultCode: Integer);
begin
  ExtractTemporaryFile(FileName);
  if not Exec(ExpandConstant('{tmp}\') + FileName, Params, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
    ResultCode := -1;
  Log(Format('Prerequisite %s %s returned %d', [FileName, Params, ResultCode]));
end;

procedure InstallPrerequisites();
var
  ResultCode: Integer;
  Usable, Problem: String;
begin
  if NeedsVCRuntime() then begin
    WizardForm.StatusLabel.Caption := 'Installing Microsoft Visual C++ Runtime...';
    RunPrerequisite('{#VCRedistExe}', '/install /quiet /norestart', ResultCode);
    // 1638: a newer version is already installed. 3010: installed, a restart is required.
    if ResultCode = 3010 then
      PrerequisiteNeedsRestart := True
    else if (ResultCode <> 0) and (ResultCode <> 1638) then begin
      PrerequisiteFailures := PrerequisiteFailures or 1;
      SuppressibleMsgBox('Microsoft Visual C++ Runtime setup failed (code ' + IntToStr(ResultCode) + '). ' +
        'Atmos Mixer Pro does not start until it is installed. ' + FreeSpaceHint(), mbError, MB_OK, IDOK);
    end;
  end else
    Log('Prerequisite Visual C++ Runtime is already installed');
  if WebView2Action = WebView2Keep then
    Log('Prerequisite WebView2 Runtime ' + WebView2Registered + ' is already installed (at least {#WebView2MinVersion})')
  else if WebView2Action = WebView2Unusable then begin
    PrerequisiteFailures := PrerequisiteFailures or 2;
    SuppressibleMsgBox('Microsoft Edge WebView2 Runtime ' + WebView2Registered + ' is installed but cannot be used. It is ' +
      'newer than the one in this setup ({#WebView2MinVersion}), so it was left as it is. The 3D room viewer is ' +
      'unavailable until it is repaired with internet access: Settings > Apps > Microsoft Edge WebView2 Runtime > Modify, ' +
      'or the installer at https://go.microsoft.com/fwlink/?linkid=2124701. The rest of the app works.', mbError, MB_OK, IDOK);
  end else begin
    WizardForm.StatusLabel.Caption := 'Installing Microsoft Edge WebView2 Runtime...';
    RunPrerequisite('{#WebView2Exe}', '/silent /install', ResultCode);
    if ResultCode <> 0 then begin
      PrerequisiteFailures := PrerequisiteFailures or 2;
      SuppressibleMsgBox('Microsoft Edge WebView2 Runtime setup failed (code ' + IntToStr(ResultCode) + '). ' +
        'The 3D room viewer is unavailable until it is installed; the rest of the app works. ' + FreeSpaceHint(),
        mbError, MB_OK, IDOK);
    end else begin
      Problem := WebView2Problem(Usable);
      if Problem <> '' then begin
        PrerequisiteFailures := PrerequisiteFailures or 2;
        SuppressibleMsgBox('Microsoft Edge WebView2 Runtime setup finished, but ' + Problem + '. The 3D room viewer is ' +
          'unavailable until it is repaired with internet access: Settings > Apps > Microsoft Edge WebView2 Runtime > ' +
          'Modify, or the installer at https://go.microsoft.com/fwlink/?linkid=2124701. The rest of the app works.',
          mbError, MB_OK, IDOK);
      end else
        Log('Prerequisite WebView2 Runtime after setup: loader uses ' + Usable);
    end;
  end;
end;

// Setup ran to completion but a prerequisite is still missing: 101 (Visual C++), 102 (WebView2) or 103 (both), so a
// silent or scripted install reports what the message boxes say. The log has the details.
function GetCustomSetupExitCode(): Integer;
begin
  Result := 0;
  if PrerequisiteFailures <> 0 then
    Result := 100 + PrerequisiteFailures;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    InstallPrerequisites();
end;

function NeedRestart(): Boolean;
begin
  Result := PrerequisiteNeedsRestart;
end;
