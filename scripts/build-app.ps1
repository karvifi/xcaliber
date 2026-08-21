param(
    [string]$Cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe",
    [string]$Output = ""
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $Output) {
    $Output = Join-Path $projectRoot 'dist\windows-app'
}
if (-not (Test-Path -LiteralPath $Cargo -PathType Leaf)) {
    throw "cargo was not found at $Cargo"
}

$target = Join-Path $projectRoot 'build\app-windows-target'
$cliOutput = Join-Path $projectRoot 'dist\windows-cli'
if (-not (Test-Path -LiteralPath (Join-Path $cliOutput 'xcaliber.exe') -PathType Leaf)) {
    & (Join-Path $PSScriptRoot 'build-windows.ps1') -Cargo $Cargo -Output $cliOutput
    if (-not $?) { throw 'Windows CLI staging failed' }
}
$env:CARGO_TARGET_DIR = $target
& $Cargo build --manifest-path (Join-Path $projectRoot 'app\src-tauri\Cargo.toml') `
    --target x86_64-pc-windows-msvc --release --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Force -Path $Output | Out-Null
Copy-Item -LiteralPath (Join-Path $target 'x86_64-pc-windows-msvc\release\xcaliber-desktop.exe') `
    -Destination (Join-Path $Output 'Xcaliber.exe') -Force
Get-ChildItem -LiteralPath $cliOutput | Copy-Item -Destination $Output -Recurse -Force

Write-Output "Windows desktop and CLI staged at $Output"
