param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $Executable).Path
function Assert-Preflight([int]$Expected) {
    $process = Start-Process -FilePath $exe -ArgumentList '--check-install-ready' -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne $Expected) { throw "Preflight returned $($process.ExitCode), expected $Expected" }
}
# Exercise the real per-user mutex without starting OSC/UI or touching Run keys.
Assert-Preflight 0
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$bytes = [byte[]]::new($sid.BinaryLength)
$sid.GetBinaryForm($bytes, 0)
$id = [BitConverter]::ToString($bytes).Replace('-', '').ToLowerInvariant()
$mutex = [Threading.Mutex]::new($false, "Local\Alcnaka.AlcMute.$id")
try { Assert-Preflight 2 } finally { $mutex.Dispose() }
Assert-Preflight 0
Write-Output 'Installer preflight: idle, running, and closed states verified.'
