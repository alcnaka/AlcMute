param([string]$TargetRoot = 'target', [string]$Iscc = 'ISCC.exe')
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot)
try {
    $version = (cargo metadata --no-deps --locked --format-version 1 | ConvertFrom-Json).packages | Where-Object name -eq 'alcmute' | Select-Object -ExpandProperty version
    if ($LASTEXITCODE -ne 0) { throw 'metadata failed' }
    foreach ($variant in @('portable','installer')) {
        $buildArgs = @('build','--release','--locked','--target-dir',"$TargetRoot/$variant")
        if ($variant -eq 'installer') { $buildArgs += @('--features','installer') }
        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) { throw "Build failed: $variant" }
        & "$PSScriptRoot/verify-version-info.ps1" -Executable "$TargetRoot/$variant/release/alcmute.exe"
        if ($LASTEXITCODE -ne 0) { throw 'VERSIONINFO failed' }
    }
    $stage = Join-Path $PWD 'dist/portable'
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath "$TargetRoot/portable/release/alcmute.exe",'README.md','CHANGELOG.md' -Destination $stage -Force
    Compress-Archive -LiteralPath "$stage/alcmute.exe","$stage/README.md","$stage/CHANGELOG.md" -DestinationPath "dist/AlcMute-$version-Portable.zip" -Force
    $buildDir = [IO.Path]::GetFullPath((Join-Path $PWD "$TargetRoot/installer/release"))
    & $Iscc "/DAppVersion=$version" "/DBuildDir=$buildDir" 'installer/AlcMute.iss'
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed' }
    $hashes = foreach ($name in @("AlcMute-$version-Setup.exe","AlcMute-$version-Portable.zip")) {
        $hash = (Get-FileHash -LiteralPath "dist/$name" -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $name"
    }
    [IO.File]::WriteAllText((Join-Path $PWD 'dist/SHA256SUMS.txt'),($hashes -join "`n")+"`n",[Text.UTF8Encoding]::new($false))
} finally { Pop-Location }
