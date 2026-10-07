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

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#ReleaseDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; The supervisor is copied next to the app by the build (CI copies it into the Release folder).
Source: "{#ReleaseDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#SupervisorExeName}"; IconFilename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "AtmosMixerPro"; ValueData: """{app}\{#SupervisorExeName}"" --logon"; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#SupervisorExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Stop the supervisor first. If the app is stopped first, the supervisor treats it as a crash and starts it again.
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#SupervisorExeName}"; Flags: runhidden; RunOnceId: "StopSupervisor"
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#MyAppExeName}"; Flags: runhidden; RunOnceId: "StopApp"

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
