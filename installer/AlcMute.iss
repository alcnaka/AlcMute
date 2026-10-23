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
UninstallDisplayIcon={app}\alcmute-logo.ico
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
Source: "{#BuildDir}\alcmute.exe"; DestName: "alcmute-preflight.exe"; Flags: dontcopy noencryption
Source: "{#BuildDir}\alcmute.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\app.ico"; DestDir: "{app}"; DestName: "alcmute-logo.ico"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
[Icons]
Name: "{group}\AlcMute"; Filename: "{app}\alcmute.exe"; WorkingDir: "{app}"; IconFilename: "{app}\alcmute-logo.ico"; IconIndex: 0
Name: "{autodesktop}\AlcMute"; Filename: "{app}\alcmute.exe"; Tasks: desktopicon; IconFilename: "{app}\alcmute-logo.ico"; IconIndex: 0
[Registry]
Root: HKCU; Subkey: "Software\Alcnaka\AlcMute"; ValueType: string; ValueName: "InstallDir"; ValueData: "{app}"
[Code]
procedure NotifyShellItem(EventID: Integer; Flags: Cardinal; Path: String; Unused: Integer);
  external 'SHChangeNotify@shell32.dll stdcall';

procedure RefreshShortcutIcons;
var Shortcut: String;
begin
  { Notify Windows of the changed shortcut and icon, without restarting Explorer. }
  NotifyShellItem($2000, $1005, ExpandConstant('{app}\alcmute-logo.ico'), 0);
  Shortcut := ExpandConstant('{group}\AlcMute.lnk');
  if FileExists(Shortcut) then NotifyShellItem($2000, $1005, Shortcut, 0);
  Shortcut := ExpandConstant('{autodesktop}\AlcMute.lnk');
  if FileExists(Shortcut) then NotifyShellItem($2000, $1005, Shortcut, 0);
end;

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
var Path, RegisteredDir, SettingsDir: String; Managed: Boolean; Code: Integer;
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
  if Result <> '' then Exit;
  { Use the same per-user mutex as older Portable versions, before copying files. }
  ExtractTemporaryFile('alcmute-preflight.exe');
  if not Exec(ExpandConstant('{tmp}\alcmute-preflight.exe'), '--check-install-ready', '', SW_HIDE, ewWaitUntilTerminated, Code) then
    Result := 'AlcMuteの起動状態を確認できませんでした。セットアップを再実行してください。'
  else if Code = 2 then
    Result := 'AlcMuteが起動中です。旧Portable版を含むAlcMuteをトレイメニューから終了して、再試行してください。'
  else if Code <> 0 then
    Result := 'AlcMuteの起動状態を確認できませんでした。ほかのAlcMuteを終了して再試行してください。';
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
  if CurStep = ssPostInstall then begin
    RefreshShortcutIcons;
    { [Run] entries execute BEFORE ssPostInstall. Start the app only after migration. }
    if not Exec(ExpandConstant('{app}\alcmute.exe'), '--install-migrate', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
      RaiseException('自動起動の移行に失敗しました。既存の登録を確認し、AlcMuteのログを参照してください。');
    { Silent updates are restarted by the update helper after verification. }
    if not WizardSilent then
      if not Exec(ExpandConstant('{app}\alcmute.exe'), '--background', '', SW_HIDE, ewNoWait, Code) then
        MsgBox('インストールは完了しましたが、AlcMuteを起動できませんでした。スタートメニューから起動してください。', mbError, MB_OK);
  end;
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
