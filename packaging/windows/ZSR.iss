; Compile twice with Edition=Full and Edition=Lite; both upgrade the same ZSR installation.
#ifndef Edition
  #error Edition must be specified by the packaging script
#endif
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

[Setup]
AppId={{35D7D6AF-E270-4D87-ACBD-9EAE31AA3CE0}
AppName=ZSR
AppVersion={#AppVersion}
AppVerName=ZSR {#AppVersion} ({#Edition})
DefaultDirName={localappdata}\Programs\ZSR
DefaultGroupName=ZSR
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputPath}
OutputBaseFilename=ZSR-{#AppVersion}-Windows-x64-{#Edition}-Setup
SetupIconFile={#SourceRoot}\logo\windows\zsr.ico
UninstallDisplayIcon={app}\ZSR.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[InstallDelete]
; Only the installer-owned engine directory is replaced when switching editions.
Type: filesandordirs; Name: "{app}\ffmpeg"

[Files]
Source: "{#AppBinary}"; DestDir: "{app}"; DestName: "ZSR.exe"; Flags: ignoreversion
#if Edition == "Full"
Source: "{#FFmpegRoot}\*"; DestDir: "{app}\ffmpeg"; Flags: ignoreversion recursesubdirs createallsubdirs
#endif

[Icons]
Name: "{group}\ZSR"; Filename: "{app}\ZSR.exe"
Name: "{autodesktop}\ZSR"; Filename: "{app}\ZSR.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\ZSR.exe"; Description: "Launch ZSR"; Flags: nowait postinstall skipifsilent
