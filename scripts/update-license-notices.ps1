# Regenerate the embedded Windows dependency notices after updating Cargo.lock.
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot)
try {
    $metadata = cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $builder = [Text.StringBuilder]::new()
    [void]$builder.AppendLine('AlcMute — Third-party licenses')
    [void]$builder.AppendLine('Generated from Cargo.lock dependencies for Windows x64. Each dependency retains its own license.')
    foreach ($package in ($metadata.packages | Sort-Object name, version)) {
        if ($package.name -in @('alcmute', 'alcmute-core')) { continue }
        [void]$builder.AppendLine("`n========================================`n$($package.name) $($package.version)")
        [void]$builder.AppendLine("License: $($package.license)")
        $root = Split-Path $package.manifest_path
        $files = @(Get-ChildItem -LiteralPath $root -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|AUTHORS)' })
        foreach ($name in @('LICENSES', 'licenses')) {
            $directory = Join-Path $root $name
            if (Test-Path -LiteralPath $directory) {
                $files += Get-ChildItem -LiteralPath $directory -File -Recurse
                break
            }
        }
        if ($package.license_file) {
            $files += Get-Item -LiteralPath (Join-Path $root $package.license_file)
        }
        if ($files.Count -eq 0) {
            $group = switch -Regex ($package.name) {
                '^clipboard-win$' { 'clipboard-win'; break }
                '^(gl_generator|khronos_api)$' { 'gl-rs'; break }
                '^lyon_' { 'lyon'; break }
                '^profiling$' { 'profiling'; break }
                '^skia-' { 'rust-skia'; break }
                '^spirv$' { 'rspirv'; break }
                '^taffy$' { 'taffy'; break }
                default { $package.name }
            }
            $override = Join-Path $PWD "assets/license-overrides/$group.txt"
            if (Test-Path -LiteralPath $override) {
                [void]$builder.AppendLine([IO.File]::ReadAllText($override))
                continue
            }
        }
        foreach ($file in ($files | Sort-Object FullName -Unique)) {
            [void]$builder.AppendLine("`n--- $([IO.Path]::GetRelativePath($root, $file.FullName)) ---`n")
            [void]$builder.AppendLine([IO.File]::ReadAllText($file.FullName))
        }
        if ($files.Count -eq 0) {
            throw "No license text found for $($package.name) $($package.version). Collect its notices before releasing."
        }
    }
    [IO.File]::WriteAllText((Join-Path $PWD 'assets/dependency-licenses.txt'), $builder.ToString().Replace("`r`n", "`n"), [Text.UTF8Encoding]::new($false))
} finally {
    Pop-Location
}
