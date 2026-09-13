# Build the Flutter Windows release app with the Rust cdylib in place.
#
# The app loads `mhy_qrscanner_bridge.dll` from beside `mhy_QRscanner.exe`, so a plain
# `flutter build windows` produces a binary that cannot start. This script is
# the supported entry point: it builds the Rust side first, then Flutter, then
# drops the cdylib into the release folder.
#
# Usage:
#   pwsh -File app\tool\build_windows.ps1
#   pwsh -File app\tool\build_windows.ps1 -SkipRust      # cdylib already built
#   pwsh -File app\tool\build_windows.ps1 -SkipFlutter   # only refresh the dll
[CmdletBinding()]
param(
    # Path to flutter.bat. Defaults to PATH, then C:\flutter.
    [string]$Flutter,
    # ASCII path to build through. Flutter's Windows build mis-decodes
    # non-ASCII project paths, so the repo is reached via a junction when needed.
    [string]$AsciiPath = 'C:\mhy-qrscanner',
    [switch]$SkipRust,
    [switch]$SkipFlutter
)

$ErrorActionPreference = 'Stop'

$scriptDir = $PSScriptRoot                                  # <repo>\app\tool
$appDir = Split-Path -Parent $scriptDir                      # <repo>\app
$repoRoot = Split-Path -Parent $appDir                       # <repo>
$dllName = 'mhy_qrscanner_bridge.dll'
$builtDll = Join-Path $repoRoot "target\release\$dllName"

function Resolve-Flutter {
    if ($Flutter) {
        if (-not (Test-Path $Flutter)) { throw "flutter not found at $Flutter" }
        return $Flutter
    }
    $onPath = Get-Command flutter.bat -ErrorAction SilentlyContinue
    if ($onPath) { return $onPath.Source }
    foreach ($candidate in @('C:\flutter\bin\flutter.bat', 'C:\src\flutter\bin\flutter.bat')) {
        if (Test-Path $candidate) { return $candidate }
    }
    throw 'Flutter SDK not found; pass -Flutter <path to flutter.bat>'
}

# `vswhere.exe` discovery needs these; some minimal Windows installs lack them.
function Initialize-BuildEnvironment {
    foreach ($pair in @(
            @{ Name = 'ProgramFiles(x86)'; Value = 'C:\Program Files (x86)' },
            @{ Name = 'ProgramW6432';      Value = 'C:\Program Files' }
        )) {
        if (-not [Environment]::GetEnvironmentVariable($pair.Name)) {
            if (Test-Path $pair.Value) {
                [Environment]::SetEnvironmentVariable($pair.Name, $pair.Value)
                Write-Host "set $($pair.Name)=$($pair.Value) (was missing)"
            }
        }
    }
}

# Flutter's MSBuild step cannot read a non-ASCII project path, so build through
# an ASCII junction. Junction creation does not need administrator rights.
function Resolve-BuildPath {
    if ($appDir -match '^[\x00-\x7F]+$') { return $appDir }
    if (Test-Path $AsciiPath) {
        $existing = (Get-Item $AsciiPath).Target
        if ($existing -and ($existing | Select-Object -First 1) -ne $appDir) {
            throw "$AsciiPath already points at $existing, not $appDir"
        }
        Write-Host "using existing junction $AsciiPath"
        return $AsciiPath
    }
    New-Item -ItemType Junction -Path $AsciiPath -Target $appDir | Out-Null
    Write-Host "created junction $AsciiPath -> $appDir"
    return $AsciiPath
}

if (-not $SkipRust) {
    Write-Host '==> cargo build --release (mhy-qrscanner-bridge, mhy-qrscanner-cli)'
    Push-Location $repoRoot
    try {
        & cargo build --release -p mhy-qrscanner-bridge -p mhy-qrscanner-cli
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
    }
    finally { Pop-Location }
}

if (-not (Test-Path $builtDll)) {
    throw "$builtDll not found; run without -SkipRust"
}

$releaseDir = Join-Path $appDir 'build\windows\x64\runner\Release'

if (-not $SkipFlutter) {
    Initialize-BuildEnvironment
    $flutterExe = Resolve-Flutter
    $buildPath = Resolve-BuildPath

    Write-Host "==> flutter pub get + build windows --release (from $buildPath)"
    Push-Location $buildPath
    try {
        & $flutterExe pub get
        if ($LASTEXITCODE -ne 0) { throw "flutter pub get failed ($LASTEXITCODE)" }
        # --no-tree-shake-icons: the icon font subsetter has been the prime
        # suspect for rail/screen icons silently missing in the packaged app;
        # shipping the full MaterialIcons font removes that whole class.
        & $flutterExe build windows --release --no-tree-shake-icons
        if ($LASTEXITCODE -ne 0) { throw "flutter build windows failed ($LASTEXITCODE)" }
    }
    finally { Pop-Location }
}

if (-not (Test-Path $releaseDir)) {
    throw "$releaseDir not found; run without -SkipFlutter"
}

Copy-Item -Path $builtDll -Destination (Join-Path $releaseDir $dllName) -Force
Write-Host ''
Write-Host 'Packaged:'
Get-ChildItem $releaseDir -Filter 'mhy_*' |
    ForEach-Object { Write-Host ("  {0}  ({1:N0} bytes)" -f $_.FullName, $_.Length) }
$exe = Join-Path $releaseDir 'mhy_QRscanner.exe'
if (-not (Test-Path $exe)) { throw "mhy_QRscanner.exe missing from $releaseDir" }
Write-Host ''
Write-Host "Run: $exe"
