param([string]$TargetRoot = 'target/distribution-build', [string]$Iscc = 'ISCC.exe')
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot)
try {
    $version = (cargo metadata --no-deps --locked --format-version 1 | ConvertFrom-Json).packages | Where-Object name -eq 'alcmute' | Select-Object -ExpandProperty version
    if ($LASTEXITCODE -ne 0) { throw 'metadata failed' }
    foreach ($variant in @('portable','installer')) {
        $buildArgs = @('build','--release','--locked','--target-dir',$TargetRoot)
        if ($variant -eq 'installer') { $buildArgs += @('--features','installer') }
        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) { throw "Build failed: $variant" }
        & "$PSScriptRoot/verify-artifact.ps1" -Executable "$TargetRoot/release/alcmute.exe" -Distribution $variant
        if ($LASTEXITCODE -ne 0) { throw 'VERSIONINFO failed' }
        $variantDir = Join-Path $PWD "dist/build/$variant"
        New-Item -ItemType Directory -Path $variantDir -Force | Out-Null
        Copy-Item -LiteralPath "$TargetRoot/release/alcmute.exe" -Destination $variantDir -Force
    }
    $stage = Join-Path $PWD 'dist/portable'
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath 'dist/build/portable/alcmute.exe','README.md','CHANGELOG.md' -Destination $stage -Force
    Compress-Archive -LiteralPath "$stage/alcmute.exe","$stage/README.md","$stage/CHANGELOG.md" -DestinationPath "dist/AlcMute-$version-Portable.zip" -Force
    $buildDir = Join-Path $PWD 'dist/build/installer'
    & $Iscc "/DAppVersion=$version" "/DBuildDir=$buildDir" 'installer/AlcMute.iss'
    if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed' }
    $setupInfo=[Diagnostics.FileVersionInfo]::GetVersionInfo((Resolve-Path -LiteralPath "dist/AlcMute-$version-Setup.exe").Path)
    # Inno reserves space in its VERSIONINFO strings by padding with spaces.
    if ($setupInfo.CompanyName.TrimEnd() -cne 'Alcnaka' -or $setupInfo.ProductName.TrimEnd() -cne 'AlcMute' -or $setupInfo.ProductVersion.TrimEnd() -cne $version) {throw 'Setup product/version mismatch'}
    $archive=[IO.Compression.ZipFile]::OpenRead((Join-Path $PWD "dist/AlcMute-$version-Portable.zip"))
    try {
        if (($archive.Entries.FullName | Sort-Object) -join ',' -cne 'alcmute.exe,CHANGELOG.md,README.md') {throw 'Unexpected portable ZIP contents'}
        $entry=$archive.GetEntry('alcmute.exe').Open()
        $algorithm=[Security.Cryptography.SHA256]::Create()
        try {$archiveHash=[BitConverter]::ToString($algorithm.ComputeHash($entry)).Replace('-','')} finally {$entry.Dispose();$algorithm.Dispose()}
        if ($archiveHash -cne (Get-FileHash -LiteralPath 'dist/build/portable/alcmute.exe' -Algorithm SHA256).Hash) {throw 'Wrong executable in portable ZIP'}
    } finally {$archive.Dispose()}
    $hashes = foreach ($name in @("AlcMute-$version-Setup.exe","AlcMute-$version-Portable.zip")) {
        $hash = (Get-FileHash -LiteralPath "dist/$name" -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $name"
    }
    [IO.File]::WriteAllText((Join-Path $PWD 'dist/SHA256SUMS.txt'),($hashes -join "`n")+"`n",[Text.UTF8Encoding]::new($false))
} finally { Pop-Location }
