param([string]$Executable = 'target/release/alcmute.exe')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot
$metadata = cargo metadata --manifest-path (Join-Path $root 'Cargo.toml') --no-deps --locked --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cannot read Cargo version' }
$version = ($metadata.packages | Where-Object name -eq 'alcmute').version
if (-not $version) { throw 'AlcMute package version not found' }
$info = [Diagnostics.FileVersionInfo]::GetVersionInfo((Resolve-Path -LiteralPath $Executable).Path)
$expected = @{
    CompanyName = 'Alcnaka'
    ProductName = 'AlcMute'
    FileDescription = 'AlcMute - VRC Background Mute'
    InternalName = 'alcmute'
    OriginalFilename = 'alcmute.exe'
    LegalCopyright = 'Copyright (c) 2026 Alcnaka'
    FileVersion = $version
    ProductVersion = $version
}
foreach ($field in $expected.Keys) {
    if ($info.$field -cne $expected[$field]) {
        throw "$field mismatch: expected '$($expected[$field])', got '$($info.$field)'"
    }
}
$parts = $version.Split('-')[0].Split('+')[0].Split('.')
foreach ($prefix in @('File', 'Product')) {
    $numeric = @($info."${prefix}MajorPart", $info."${prefix}MinorPart", $info."${prefix}BuildPart", $info."${prefix}PrivatePart") -join '.'
    if ($numeric -ne ($parts + '0' -join '.')) { throw "$prefix numeric version mismatch: $numeric" }
}
$info | Select-Object CompanyName,ProductName,FileDescription,InternalName,OriginalFilename,LegalCopyright,FileVersion,ProductVersion
