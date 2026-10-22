param([Parameter(Mandatory)][string]$Executable,[ValidateSet('portable','installer')][string]$Distribution)
$ErrorActionPreference='Stop'
& "$PSScriptRoot/verify-version-info.ps1" -Executable $Executable
$exe=(Resolve-Path -LiteralPath $Executable).Path
$info=[Diagnostics.FileVersionInfo]::GetVersionInfo($exe)
if ($Distribution -and $info.Comments -ine $Distribution) { throw 'Distribution resource mismatch' }
if (-not ('AlcMuteResourceProbe' -as [type])) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class AlcMuteResourceProbe {
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] public static extern IntPtr LoadLibraryEx(string path,IntPtr file,uint flags);
 [DllImport("kernel32.dll",SetLastError=true)] public static extern IntPtr FindResource(IntPtr module,IntPtr name,IntPtr type);
 [DllImport("kernel32.dll")] public static extern IntPtr LoadResource(IntPtr module,IntPtr resource);
 [DllImport("kernel32.dll")] public static extern IntPtr LockResource(IntPtr resource);
 [DllImport("kernel32.dll")] public static extern uint SizeofResource(IntPtr module,IntPtr resource);
 [DllImport("kernel32.dll")] public static extern bool FreeLibrary(IntPtr module);
}
'@
}
$module=[AlcMuteResourceProbe]::LoadLibraryEx($exe,[IntPtr]::Zero,2)
if ($module -eq [IntPtr]::Zero) { throw 'Cannot load EXE resources' }
try {
    $resource=[AlcMuteResourceProbe]::FindResource($module,[IntPtr]1,[IntPtr]14)
    if ($resource -eq [IntPtr]::Zero) { throw 'Missing product icon group' }
    $size=[AlcMuteResourceProbe]::SizeofResource($module,$resource)
    $bytes=[byte[]]::new($size)
    [Runtime.InteropServices.Marshal]::Copy([AlcMuteResourceProbe]::LockResource([AlcMuteResourceProbe]::LoadResource($module,$resource)),$bytes,0,$bytes.Length)
    $count=[BitConverter]::ToUInt16($bytes,4)
    $dimensions=for ($index=0;$index -lt $count;$index++) { $value=$bytes[6+14*$index]; if ($value -eq 0) {256} else {[int]$value} }
    foreach ($required in @(16,24,32,48,256)) {if ($required -notin $dimensions) {throw "Missing icon size: $required"}}
} finally { [void][AlcMuteResourceProbe]::FreeLibrary($module) }
$binary=[Text.Encoding]::UTF8.GetString([IO.File]::ReadAllBytes($exe))
if (-not $binary.Contains([IO.File]::ReadAllText((Join-Path (Split-Path $PSScriptRoot) 'LICENSE')))) {throw 'Missing MIT license'}
if ($binary.Contains('/alcmute/test/')) {throw 'Development endpoints in production artifact'}
$process=Start-Process -FilePath $exe -ArgumentList '--verify-licenses' -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) {throw 'Compressed license integrity verification failed'}
Write-Output "Verified $Distribution artifact: VERSIONINFO, icon sizes, license decompression, production endpoints."
