#define MyAppName "Atmos Mixer Pro"
#define MyAppPublisher "Atmos"
#define MyAppExeName "atmos_mixer_pro.exe"
; The version is read from the built exe. Flutter writes pubspec.yaml's version into the exe's
; ProductVersion, so the installer always matches the app. The path uses the same base as [Files].
#define MyAppExePath AddBackslash(SourcePath) + "build\windows\x64\runner\Release\" + MyAppExeName
#define MyAppVersion GetStringFileInfo(MyAppExePath, "ProductVersion")
#if MyAppVersion == ""
  #error Could not read the version from the built exe. Run "flutter build windows" first.
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
Source: "build\windows\x64\runner\Release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "build\windows\x64\runner\Release\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "AtmosMixerPro"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
