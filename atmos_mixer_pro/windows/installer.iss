; Folder with the built app (Release folder + supervisor exe). Default: the repository build, one level up
; from this script. For the CI zip: ISCC /DReleaseDir=<folder> windows\installer.iss
#ifndef ReleaseDir
  #define ReleaseDir AddBackslash(SourcePath) + "..\build\windows\x64\runner\Release"
#endif
#define MyAppName "Atmos Mixer Pro"
#define MyAppPublisher "Atmos"
#define MyAppExeName "atmos_mixer_pro.exe"
; The supervisor relaunches the app after a crash or a hang. Shortcuts, the logon Run key and the
; post-install launch start the supervisor, which then starts the app.
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
#define VCRedistExe "VC_redist.x64.exe"
#define WebView2Exe "MicrosoftEdgeWebView2RuntimeInstallerX64.exe"
#if !FileExists(AddBackslash(RedistDir) + VCRedistExe) || !FileExists(AddBackslash(RedistDir) + WebView2Exe)
  #error Missing Microsoft redistributables. Run tool\windows\fetch-redist.ps1 first or pass /DRedistDir.
#endif
#define VCRedistVersion GetVersionNumbersString(AddBackslash(RedistDir) + VCRedistExe)
; The WebView2 installer's file version is Edge Update's (1.3.x), not the runtime's, so the runtime it carries is
; written here: 154.0.4258.62 in the one fetch-redist.ps1 got on 2026-10-08 (it always gets the latest). A registered
; runtime at least this new is kept. Never set it above the bundled runtime: a registered runtime newer than the
; bundled one would then run the bundled installer, which can fail on it.
#define WebView2MinVersion "154.0.4258.62"

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

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#ReleaseDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; The supervisor is copied next to the app by the build (CI copies it into the Release folder).
Source: "{#ReleaseDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; Extracted to {tmp} only when needed (InstallPrerequisites).
Source: "{#RedistDir}\{#VCRedistExe}"; Flags: dontcopy
Source: "{#RedistDir}\{#WebView2Exe}"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "AtmosMixerPro"; ValueData: """{app}\{#SupervisorExeName}"" --logon"; Flags: uninsdeletevalue

[Run]
; OSC arrives over UDP on 0.0.0.0. Allow it here instead of the first-run firewall prompt, which nobody answers
; on an unattended PC: any sender on private and domain networks, only the local subnet on public networks.
; Rules for this exe that the prompt may have left (allow or block) are removed first.
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=all program=""{app}\{#MyAppExeName}"""; Flags: runhidden waituntilterminated
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall add rule name=""Atmos Mixer Pro OSC"" dir=in action=allow program=""{app}\{#MyAppExeName}"" protocol=UDP profile=private,domain enable=yes"; Flags: runhidden waituntilterminated
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall add rule name=""Atmos Mixer Pro OSC (public, local subnet)"" dir=in action=allow program=""{app}\{#MyAppExeName}"" protocol=UDP profile=public remoteip=localsubnet enable=yes"; Flags: runhidden waituntilterminated
Filename: "{app}\{#SupervisorExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Stop the supervisor first. If the app is stopped first, the supervisor treats it as a crash and starts it again.
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#SupervisorExeName}"; Flags: runhidden; RunOnceId: "StopSupervisor"
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#MyAppExeName}"; Flags: runhidden; RunOnceId: "StopApp"
Filename: "{sys}\netsh.exe"; Parameters: "advfirewall firewall delete rule name=all program=""{app}\{#MyAppExeName}"""; Flags: runhidden; RunOnceId: "DeleteFirewallRules"

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

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopAtmos();
  Result := '';
end;

const
  VCRuntimeKey = 'SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64';
  WebView2Key = 'Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';

var
  PrerequisiteNeedsRestart: Boolean;

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

// The runtime registers per machine or per user. The bundled installer runs when none is registered or the newest one
// is older than the bundled runtime. Registered alone was not enough: Windows Sandbox (Z6, 2026-10-11) registered a
// 141 runtime with its files that the WebView2 loader could not use, so the 3D room viewer did not start; being older
// than the bundled runtime, it is now replaced. The same or a newer runtime is kept: over a newer one the bundled
// installer fails (-2147219187 = 0x8004090D in Z6), which would show a false failure.
function NeedsWebView2(var Registered: String): Boolean;
var
  Newest, MinVersion: Int64;
begin
  Newest := 0;
  Registered := 'none';
  TakeNewerWebView2(HKLM32, 'SOFTWARE\' + WebView2Key, Newest, Registered);
  TakeNewerWebView2(HKCU, 'Software\' + WebView2Key, Newest, Registered);
  Result := not StrToVersion('{#WebView2MinVersion}', MinVersion) or (ComparePackedVersion(Newest, MinVersion) < 0);
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
  RegisteredWebView2: String;
begin
  if NeedsVCRuntime() then begin
    WizardForm.StatusLabel.Caption := 'Installing Microsoft Visual C++ Runtime...';
    RunPrerequisite('{#VCRedistExe}', '/install /quiet /norestart', ResultCode);
    // 1638: a newer version is already installed. 3010: installed, a restart is required.
    if ResultCode = 3010 then
      PrerequisiteNeedsRestart := True
    else if (ResultCode <> 0) and (ResultCode <> 1638) then
      SuppressibleMsgBox('Microsoft Visual C++ Runtime setup failed (code ' + IntToStr(ResultCode) + '). ' +
        'Atmos Mixer Pro does not start until it is installed.', mbError, MB_OK, IDOK);
  end else
    Log('Prerequisite Visual C++ Runtime is already installed');
  if NeedsWebView2(RegisteredWebView2) then begin
    Log('Prerequisite WebView2 Runtime registered: ' + RegisteredWebView2 + ' (older than {#WebView2MinVersion} or none)');
    WizardForm.StatusLabel.Caption := 'Installing Microsoft Edge WebView2 Runtime...';
    RunPrerequisite('{#WebView2Exe}', '/silent /install', ResultCode);
    if ResultCode <> 0 then
      SuppressibleMsgBox('Microsoft Edge WebView2 Runtime setup failed (code ' + IntToStr(ResultCode) + '). ' +
        'The 3D room viewer is unavailable until it is installed; the rest of the app works.', mbError, MB_OK, IDOK);
  end else
    Log('Prerequisite WebView2 Runtime ' + RegisteredWebView2 + ' is already installed (at least {#WebView2MinVersion})');
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
