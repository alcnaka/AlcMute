#ifndef AppVersion
  #define AppVersion "0.3.0"
#endif
#ifndef BuildDir
  #define BuildDir "..\target\installer\release"
#endif
[Setup]
AppId={{D7267BA5-A7BD-46DB-A39F-47E9F7843CF6}
AppName=AlcMute
AppVersion={#AppVersion}
VersionInfoVersion={#AppVersion}
VersionInfoCompany=Alcnaka
VersionInfoProductName=AlcMute
VersionInfoDescription=AlcMute Setup
AppPublisher=Alcnaka
AppPublisherURL=https://github.com/alcnaka/AlcMute
DefaultDirName={localappdata}\Programs\Alcnaka\AlcMute
DefaultGroupName=AlcMute
PrivilegesRequired=lowest
UsePreviousAppDir=yes
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
SetupIconFile=..\assets\app.ico
UninstallDisplayIcon={app}\alcmute.exe
OutputDir=..\dist
OutputBaseFilename=AlcMute-{#AppVersion}-Setup
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
AppMutex=Local\Alcnaka.AlcMute.Installation
RestartApplications=no
CloseApplicationsFilter=alcmute.exe
WizardStyle=modern
[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
[Tasks]
Name: "desktopicon"; Description: "デスクトップにショートカットを作成する"; Flags: unchecked
[Files]
Source: "{#BuildDir}\alcmute.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
[Icons]
Name: "{group}\AlcMute"; Filename: "{app}\alcmute.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\AlcMute"; Filename: "{app}\alcmute.exe"; Tasks: desktopicon
[Registry]
Root: HKCU; Subkey: "Software\Alcnaka\AlcMute"; ValueType: string; ValueName: "InstallDir"; ValueData: "{app}"
[Run]
Filename: "{app}\alcmute.exe"; Parameters: "--background"; Flags: nowait skipifdoesntexist skipifsilent
[Code]
function DirectoryHasEntries(const Path: String): Boolean;
var Entry: TFindRec;
begin
  Result := False;
  if FindFirst(AddBackslash(Path) + '*', Entry) then begin
    try
      repeat
        if (Entry.Name <> '.') and (Entry.Name <> '..') then begin
          Result := True;
          Break;
        end;
      until not FindNext(Entry);
    finally
      FindClose(Entry);
    end;
  end;
end;
function PrepareToInstall(var NeedsRestart: Boolean): String;
var Path, RegisteredDir, SettingsDir: String; Managed: Boolean;
begin
  Result := '';
  Path := AddBackslash(ExpandFileName(WizardDirValue));
  SettingsDir := AddBackslash(ExpandConstant('{localappdata}\Alcnaka\AlcMute'));
  if Pos(Lowercase(AddBackslash(ExpandConstant('{localappdata}'))),
    Lowercase(Path)) <> 1 then
    Result := '管理者権限なしで更新するため、ユーザーのLocalAppData配下を選択してください。';
  if Pos(Lowercase(SettingsDir), Lowercase(Path)) = 1 then
    Result := '設定保存フォルダーとプログラムのインストール先は別にしてください。';
  Managed := RegQueryStringValue(HKCU, 'Software\Alcnaka\AlcMute', 'InstallDir', RegisteredDir);
  Managed := Managed and (CompareText(AddBackslash(RegisteredDir), Path) = 0);
  if (not Managed) and DirectoryHasEntries(Path) then
    Result := '既存のPortable版や他のファイルを上書きしないよう、空の専用フォルダーを選択してください。';
end;
function NextButtonClick(CurPageID: Integer): Boolean;
var Path: String;
begin
  Result := True;
  if CurPageID = wpSelectDir then begin
    Path := Lowercase(AddBackslash(ExpandFileName(WizardDirValue)));
    { Constrain to the current user's LocalAppData tree; never elevate for updates. }
    if Pos(Lowercase(AddBackslash(ExpandConstant('{localappdata}'))), Path) <> 1 then begin
      MsgBox('管理者権限なしで更新するため、ユーザーのLocalAppData配下を選択してください。', mbError, MB_OK);
      Result := False;
    end;
  end;
end;
procedure CurStepChanged(CurStep: TSetupStep);
var Code: Integer;
begin
  if CurStep = ssPostInstall then
    if not Exec(ExpandConstant('{app}\alcmute.exe'), '--install-migrate', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
      RaiseException('自動起動の移行に失敗しました。既存の登録を確認し、AlcMuteのログを参照してください。');
end;
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Dir: String; Code: Integer;
begin
  if CurUninstallStep = usUninstall then
    if not Exec(ExpandConstant('{app}\alcmute.exe'), '--uninstall-cleanup', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
      RaiseException('自動起動を解除できませんでした。AlcMuteを終了して再試行してください。');
  if CurUninstallStep = usPostUninstall then
    if RegQueryStringValue(HKCU, 'Software\Alcnaka\AlcMute', 'InstallDir', Dir) and
      (CompareText(RemoveBackslashUnlessRoot(Dir), RemoveBackslashUnlessRoot(ExpandConstant('{app}'))) = 0) then
      RegDeleteValue(HKCU, 'Software\Alcnaka\AlcMute', 'InstallDir');
end;
