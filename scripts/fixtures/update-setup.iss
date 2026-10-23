; Test-only installer. No AlcMute AppId, startup entry, shortcut, or user settings.
[Setup]
AppId=AlcMuteLocalUpdateFixture-{#TestId}
AppName=AlcMute Local Update Fixture
AppVersion=0.0.2
DefaultDirName={#FixtureRoot}\install
PrivilegesRequired=lowest
UsePreviousAppDir=no
DisableDirPage=yes
DisableProgramGroupPage=yes
OutputDir={#FixtureRoot}
OutputBaseFilename=AlcMute-0.0.2-Setup
Compression=lzma2
CloseApplications=no
RestartApplications=no
[Files]
Source: "{#FixtureRoot}\new.exe"; DestDir: "{app}"; DestName: "alcmute.exe"; Flags: ignoreversion
[Registry]
Root: HKCU; Subkey: "Software\AlcMute\Testing\LocalUpdater\{#TestId}"; ValueType: string; ValueName: "Version"; ValueData: "0.0.2"; Flags: uninsdeletekey
[Code]
function InitializeSetup: Boolean;
begin
  Result := GetEnv('ALCMUTE_FIXTURE_FAIL') <> '1';
end;
function PrepareToInstall(var NeedsRestart: Boolean): String;
var Marker: AnsiString; Root: String;
begin
  Root := GetEnv('ALCMUTE_LOCAL_UPDATE_ROOT');
  if (Root = '') or
     (CompareText(ExpandConstant('{app}'), AddBackslash(Root) + 'install') <> 0) or
     not LoadStringFromFile(AddBackslash(Root) + 'fixture.marker', Marker) or
     (String(Marker) <> '{#TestId}') then
    Result := 'Local update fixture path/marker mismatch';
end;
