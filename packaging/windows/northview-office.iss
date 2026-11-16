; Northview Office — Windows installer (Inno Setup 6).
; Built by package.py:  ISCC /DAppVersion=<ver> /DSourceExe=<exe> /DOutputDir=<dist> northview-office.iss
; Per-user install: no administrator account needed. Upgrades in place (same AppId).

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\release\nvdental-office-app.exe"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif
#define AppName "Northview Office"
#define AppUserModelId "SturlDigital.NorthviewOffice"

[Setup]
AppId={{0D3D1619-4B2A-414C-8A69-0F2BD3967C96}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Sturl Digital Services
VersionInfoVersion={#AppVersion}
VersionInfoProductName={#AppName}
VersionInfoDescription={#AppName} installer
DefaultDirName={localappdata}\Programs\NorthviewOffice
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename=NorthviewOffice-{#AppVersion}-setup
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\NorthviewOffice.exe
UninstallDisplayName={#AppName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"

[InstallDelete]
; leftovers from the old PowerShell installer (0.1.0 – 0.1.2)
Type: files; Name: "{app}\uninstall.ps1"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "NorthviewOffice.exe"; Flags: ignoreversion
Source: "..\..\assets\icon.png"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\{#AppName}"; Filename: "{app}\NorthviewOffice.exe"; WorkingDir: "{app}"; Comment: "Northview Dental front-desk app"; AppUserModelID: "{#AppUserModelId}"
Name: "{userdesktop}\{#AppName}"; Filename: "{app}\NorthviewOffice.exe"; WorkingDir: "{app}"; Comment: "Northview Dental front-desk app"; AppUserModelID: "{#AppUserModelId}"; Tasks: desktopicon

[Registry]
; notification identity, so Windows shows the Northview name and icon on alerts
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\{#AppUserModelId}"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\{#AppUserModelId}"; ValueType: string; ValueName: "DisplayName"; ValueData: "{#AppName}"
Root: HKCU; Subkey: "Software\Classes\AppUserModelId\{#AppUserModelId}"; ValueType: string; ValueName: "IconUri"; ValueData: "{app}\icon.png"

[Run]
Filename: "{app}\NorthviewOffice.exe"; Description: "Open {#AppName} now"; Flags: nowait postinstall skipifsilent

; Settings in %LOCALAPPDATA%\NorthviewOffice are kept on uninstall.
