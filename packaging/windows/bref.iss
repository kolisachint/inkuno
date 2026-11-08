; Windows installer of Inkuno. Built by .github/workflows/release-binaries.yml:
;   INKUNO_VERSION=0.2.0 iscc packaging/windows/inkuno.iss   (makes dist/inkuno-windows-x86_64-setup.exe)
#define Version GetEnv("INKUNO_VERSION")

[Setup]
AppId={{9A5A9701-FBF2-446B-8861-E78C81C9CA55}
AppName=Inkuno
AppVersion={#Version}
AppPublisher=Andrea Larboullet Marin
AppPublisherURL=https://github.com/alarboulletmarin/bref
DefaultDirName={autopf}\Inkuno
DefaultGroupName=Inkuno
; Installed for the current user only (%LOCALAPPDATA%\Programs\Inkuno): no administrator prompt.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
MinVersion=10.0
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
SetupIconFile=inkuno.ico
UninstallDisplayIcon={app}\inkuno.exe
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=inkuno-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "..\..\target\release\inkuno.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Inkuno"; Filename: "{app}\inkuno.exe"
Name: "{autodesktop}\Inkuno"; Filename: "{app}\inkuno.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\inkuno.exe"; Description: "{cm:LaunchProgram,Inkuno}"; Flags: nowait postinstall skipifsilent
; Update started by the app (setup /VERYSILENT ... /RELAUNCH=1): the app opens again by itself.
Filename: "{app}\inkuno.exe"; Flags: nowait runasoriginaluser; Check: Relaunch

[Code]
function Relaunch: Boolean;
begin
  Result := ExpandConstant('{param:RELAUNCH|0}') = '1';
end;
