; version and exe dir are passed via /D flags:
;   iscc /DAppVersion=0.1.0 /DExeDir=..\dist SteamGridUI.iss

#ifndef ExeDir
  #define ExeDir "..\dist"
#endif

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define AppName "Steam Grid UI"
#define AppPublisher "juiced-vibing"
#define AppURL "https://github.com/juiced-vibing/steam-grid-ui"
#define AppExeName "SteamGridUI.exe"
#define RuntimeUrl "https://aka.ms/dotnet/10.0/windowsdesktop-runtime-win-x64.exe"
#define RuntimeFile "windowsdesktop-runtime-win-x64.exe"

[Setup]
AppId={{852D2861-99A8-4F99-B99A-7970F3B9AAD1}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
AppUpdatesURL={#AppURL}/releases
DefaultDirName={autopf}\SteamGridUI
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=output
OutputBaseFilename=SteamGridUI-{#AppVersion}-setup
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#AddBackslash(ExeDir)}{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
; the runtime's own installer asks for elevation, this one never does
Filename: "{tmp}\{#RuntimeFile}"; Parameters: "/install /quiet /norestart"; StatusMsg: "Installing the .NET 10 Desktop Runtime..."; Flags: waituntilterminated; Check: NeedsRuntime
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; the thumbnail cache only: the saved key and the artwork backups are the user's
Type: filesandordirs; Name: "{userappdata}\SteamGridUI\thumbs"

[Code]
var
  DownloadPage: TDownloadWizardPage;

function NeedsRuntime: Boolean;
var
  Found: TFindRec;
begin
  Result := not FindFirst(ExpandConstant('{commonpf64}\dotnet\shared\Microsoft.WindowsDesktop.App\10.*'), Found);
  if not Result then
    FindClose(Found);
end;

procedure InitializeWizard;
begin
  DownloadPage := CreateDownloadPage('Downloading the .NET 10 Desktop Runtime', 'Steam Grid UI needs it to run.', nil);
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;

  if (CurPageID <> wpReady) or not NeedsRuntime then
    Exit;

  DownloadPage.Clear;
  DownloadPage.Add('{#RuntimeUrl}', '{#RuntimeFile}', '');
  DownloadPage.Show;
  try
    try
      DownloadPage.Download;
    except
      if not DownloadPage.AbortedByUser then
        SuppressibleMsgBox(AddPeriod(GetExceptionMessage), mbCriticalError, MB_OK, IDOK);
      Result := False;
    end;
  finally
    DownloadPage.Hide;
  end;
end;
