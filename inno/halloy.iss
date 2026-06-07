#define AppName "Halloy"
#define AppPublisher "Squidowl"
#define AppExeName "halloy.exe"
#define AppId "{{3ED18662-FB43-4803-A4F9-89BBBC0B6D01}"
#define AppVersion GetEnv("HALLOY_VERSION")
#define SourceDir GetEnv("HALLOY_SOURCE_DIR")
#define OutputDir GetEnv("HALLOY_OUTPUT_DIR")
#define OutputBaseFilename GetEnv("HALLOY_OUTPUT_BASE_FILENAME")

[Setup]
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL=https://halloy.chat/
AppSupportURL=https://halloy.chat/
AppUpdatesURL=https://halloy.chat/
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
LicenseFile={#SourceDir}\wix\license.rtf
OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseFilename}
SetupIconFile={#SourceDir}\assets\windows\halloy.ico
UninstallDisplayIcon={app}\{#AppExeName}
VersionInfoVersion={#AppVersion}
VersionInfoCompany={#AppPublisher}
VersionInfoDescription={#AppName} Installer
VersionInfoProductName={#AppName}
VersionInfoProductVersion={#AppVersion}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
CloseApplications=yes
RestartApplications=no
ChangesAssociations=yes
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\target\packaging\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; AppUserModelID: "org.squidowl.halloy"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon; AppUserModelID: "org.squidowl.halloy"

[Registry]
Root: HKCU; Subkey: "Software\Halloy\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Halloy - IRC client"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Halloy\Capabilities"; ValueType: string; ValueName: "ApplicationIcon"; ValueData: "{app}\{#AppExeName},0"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Halloy\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Halloy"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Halloy\Capabilities\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExeName},1"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Halloy\Capabilities\URLAssociations"; ValueType: string; ValueName: "halloy"; ValueData: "halloy"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Halloy\Capabilities\URLAssociations"; ValueType: string; ValueName: "irc"; ValueData: "irc"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Halloy\Capabilities\URLAssociations"; ValueType: string; ValueName: "ircs"; ValueData: "ircs"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Halloy"; ValueData: "Software\Halloy\Capabilities"; Flags: uninsdeletevalue

Root: HKCU; Subkey: "Software\Classes\halloy"; ValueType: string; ValueName: ""; ValueData: "URL:Halloy"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\halloy"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "Halloy URL"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\halloy"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\halloy\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExeName},1"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\halloy\shell"; ValueType: string; ValueName: ""; ValueData: "open"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\halloy\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" ""%1"""; Flags: uninsdeletekey

Root: HKCU; Subkey: "Software\Classes\irc"; ValueType: string; ValueName: ""; ValueData: "URL:Internet Relay Chat"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\irc"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "Internet Relay Chat URL"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\irc"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\irc\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExeName},1"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\irc\shell"; ValueType: string; ValueName: ""; ValueData: "open"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\irc\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" ""%1"""; Flags: uninsdeletekey

Root: HKCU; Subkey: "Software\Classes\ircs"; ValueType: string; ValueName: ""; ValueData: "URL:Internet Relay Chat with Privacy"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ircs"; ValueType: string; ValueName: "FriendlyTypeName"; ValueData: "Internet Relay Chat with Privacy URL"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ircs"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ircs\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExeName},1"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ircs\shell"; ValueType: string; ValueName: ""; ValueData: "open"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\ircs\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" ""%1"""; Flags: uninsdeletekey

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
const
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall';

function IsHalloyMsiInstall(Root: Integer; Subkey: String): Boolean;
var
  DisplayName: String;
  Publisher: String;
  WindowsInstaller: Cardinal;
begin
  Result := False;
  WindowsInstaller := 0;

  if not RegQueryStringValue(Root, UninstallKey + '\' + Subkey, 'DisplayName', DisplayName) then
    Exit;

  if CompareText(DisplayName, 'Halloy') <> 0 then
    Exit;

  RegQueryStringValue(Root, UninstallKey + '\' + Subkey, 'Publisher', Publisher);
  RegQueryDWordValue(Root, UninstallKey + '\' + Subkey, 'WindowsInstaller', WindowsInstaller);

  Result := (CompareText(Publisher, 'Squidowl') = 0) and (WindowsInstaller = 1);
end;

function FindHalloyMsiInstall(Root: Integer; var ProductCode: String): Boolean;
var
  Subkeys: TArrayOfString;
  Index: Integer;
begin
  Result := False;

  if not RegGetSubkeyNames(Root, UninstallKey, Subkeys) then
    Exit;

  for Index := 0 to GetArrayLength(Subkeys) - 1 do
  begin
    if IsHalloyMsiInstall(Root, Subkeys[Index])
      and (Length(Subkeys[Index]) = 38)
      and (Pos('{', Subkeys[Index]) = 1)
      and (Subkeys[Index][38] = '}') then
    begin
      ProductCode := Subkeys[Index];
      Result := True;
      Exit;
    end;
  end;
end;

function UninstallHalloyMsi(ProductCode: String): Boolean;
var
  ResultCode: Integer;
begin
  Result :=
    ShellExec('runas', ExpandConstant('{sys}\msiexec.exe'), '/x ' + ProductCode + ' /passive /norestart',
      '', SW_SHOW, ewWaitUntilTerminated, ResultCode)
    and ((ResultCode = 0) or (ResultCode = 3010));
end;

procedure CloseRunningHalloy();
var
  ResultCode: Integer;
begin
  Log('Closing any running Halloy process before installing files.');
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/im {#AppExeName} /t',
    '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

function InitializeSetup(): Boolean;
var
  ProductCode: String;
  Root: Integer;
begin
  Result := True;

  if FindHalloyMsiInstall(HKLM64, ProductCode) then
    Root := HKLM64
  else if FindHalloyMsiInstall(HKLM32, ProductCode) then
    Root := HKLM32
  else
    Exit;

  Log('Found existing Halloy MSI installation in registry root ' + IntToStr(Root) + ': ' + ProductCode);

  if MsgBox('Setup found an existing Halloy MSI installation. It must be removed before installing this per-user version.' #13#13 +
            'Setup will ask Windows for permission to uninstall the existing MSI installation.',
            mbConfirmation, MB_YESNO) <> IDYES then
  begin
    Result := False;
    Exit;
  end;

  if not UninstallHalloyMsi(ProductCode) then
  begin
    MsgBox('Setup could not uninstall the existing Halloy MSI installation. Please approve the Windows permission prompt, or remove Halloy from Windows Settings and run this installer again.',
      mbError, MB_OK);
    Result := False;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
    CloseRunningHalloy();
end;
