param(
    [string]$Cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe",
    [string]$Output = ""
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $Output) {
    $Output = Join-Path $projectRoot 'dist\release'
}
if (-not (Test-Path -LiteralPath $Cargo -PathType Leaf)) {
    throw "cargo was not found at $Cargo"
}

$cliOutput = Join-Path $projectRoot 'dist\windows-cli'
& (Join-Path $PSScriptRoot 'build-windows.ps1') -Cargo $Cargo -Output $cliOutput
if (-not $?) { throw 'Windows CLI staging failed' }

& $Cargo tauri --version | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw 'Tauri CLI 2 is required. Install it with: cargo install tauri-cli --version ^2 --locked'
}

$target = Join-Path $projectRoot 'build\installer-target'
$env:CARGO_TARGET_DIR = $target
Push-Location (Join-Path $projectRoot 'app')
try {
    & $Cargo tauri build --config src-tauri/tauri.release.conf.json --target x86_64-pc-windows-msvc --bundles nsis
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}

$installerDirectory = Join-Path $target 'x86_64-pc-windows-msvc\release\bundle\nsis'
$installer = Get-ChildItem -LiteralPath $installerDirectory -File -Filter '*-setup.exe' |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
if (-not $installer) {
    throw "NSIS installer was not produced under $installerDirectory"
}
New-Item -ItemType Directory -Force -Path $Output | Out-Null
Copy-Item -LiteralPath $installer.FullName -Destination (Join-Path $Output $installer.Name) -Force
Write-Output "Windows installer staged at $Output"
