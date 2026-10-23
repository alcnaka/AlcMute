param([string]$TargetRoot = 'target', [string]$Iscc = 'ISCC.exe')
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot)
$testId = [guid]::NewGuid().ToString('D')
$testRoot = [IO.Path]::GetFullPath((Join-Path $PWD "results/local-update-tests/$testId"))
$priorRoot = $env:ALCMUTE_LOCAL_UPDATE_ROOT
$priorId = $env:ALCMUTE_LOCAL_UPDATE_ID
$priorFail = $env:ALCMUTE_FIXTURE_FAIL
try {
    $compiler = Join-Path $env:SystemRoot 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
    if (-not (Test-Path -LiteralPath $compiler)) { throw 'Offline .NET Framework C# compiler not found' }
    if (-not (Get-Command $Iscc -ErrorAction SilentlyContinue)) { throw 'Inno Setup compiler not found; supply -Iscc <ISCC.exe>' }
    New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $testRoot 'fixture.marker'), $testId)
    $env:ALCMUTE_LOCAL_UPDATE_ROOT = $testRoot
    $env:ALCMUTE_LOCAL_UPDATE_ID = $testId
    $env:ALCMUTE_FIXTURE_FAIL = $null
    $source = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'fixtures/update-app.cs'))
    foreach ($fixture in @(@('old','0.0.1'), @('new','0.0.2'))) {
        $cs = Join-Path $testRoot "$($fixture[0]).cs"
        [IO.File]::WriteAllText($cs, $source.Replace('FIXTURE_VERSION', $fixture[1]))
        & $compiler /nologo /target:winexe /platform:x64 "/out:$testRoot/$($fixture[0]).exe" $cs
        if ($LASTEXITCODE -ne 0) { throw 'Fixture compilation failed' }
    }
    & $Iscc "/DTestId=$testId" "/DFixtureRoot=$testRoot" 'scripts/fixtures/update-setup.iss'
    if ($LASTEXITCODE -ne 0) { throw 'Fixture Setup compilation failed' }
    cargo test -p alcmute-core --features windows-integration,installer --offline --locked --target-dir $TargetRoot updater::installation::local_tests::local_update_e2e -- --ignored --exact --test-threads=1 --nocapture
    if ($LASTEXITCODE -ne 0) { throw "Local update tests failed; inspect $testRoot" }
    Write-Output "Local-only update tests passed. Report and installer logs: $testRoot"
} finally {
    # These exact keys belong to this run's GUID, never the real AlcMute AppId.
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree("Software\AlcMute\Testing\LocalUpdater\$testId", $false)
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree("Software\Microsoft\Windows\CurrentVersion\Uninstall\AlcMuteLocalUpdateFixture-$($testId)_is1", $false)
    $env:ALCMUTE_LOCAL_UPDATE_ROOT = $priorRoot
    $env:ALCMUTE_LOCAL_UPDATE_ID = $priorId
    $env:ALCMUTE_FIXTURE_FAIL = $priorFail
    Pop-Location
}
