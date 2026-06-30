#define AppPublisher "DbState"
#ifndef AppVersion
#define AppVersion "v0.1.0-alpha.3"
#endif
#ifndef SourceDir
#define SourceDir "staging"
#endif
#ifndef OutputDir
#define OutputDir "out"
#endif

[Setup]
AppId={{6C3D38F7-7E2E-4DE7-9F84-8D6F530D6D01}
AppName=DbState PostgreSQL
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\DbState
DefaultGroupName=DbState
DisableProgramGroupPage=yes
OutputDir={#OutputDir}
OutputBaseFilename=DbState-PostgreSQL-{#AppVersion}-setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayName=DbState PostgreSQL

[Files]
Source: "{#SourceDir}\dbstate.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\VERSION.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\Start DbState Local Service"; Filename: "{app}\dbstate.exe"; Parameters: "serve --host 127.0.0.1 --port 4587"; WorkingDir: "{userdocs}"
Name: "{group}\Open DbState UI"; Filename: "{cmd}"; Parameters: "/c start """" ""http://127.0.0.1:4587/"""
Name: "{group}\DbState Help"; Filename: "{cmd}"; Parameters: "/k ""{app}\dbstate.exe"" --help"

[Run]
Filename: "{app}\README.txt"; Description: "View private beta readme"; Flags: postinstall shellexec skipifsilent
