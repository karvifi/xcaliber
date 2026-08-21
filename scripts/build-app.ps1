param(
    [string]$Cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe",
    [string]$Output = "",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $Output) {
    $Output = Join-Path $projectRoot 'dist\windows-app'
}
if (-not (Test-Path -LiteralPath $Cargo -PathType Leaf)) {
    throw "cargo was not found at $Cargo"
}

$appBuildDirectory = Join-Path $projectRoot 'build\app-windows-target'
$cliOutput = Join-Path $projectRoot 'dist\windows-cli'
if (-not (Test-Path -LiteralPath (Join-Path $cliOutput 'xcaliber.exe') -PathType Leaf)) {
    & (Join-Path $PSScriptRoot 'build-windows.ps1') -Cargo $Cargo -Output $cliOutput
    if (-not $?) { throw 'Windows CLI staging failed' }
}
$env:CARGO_TARGET_DIR = $appBuildDirectory
& $Cargo build --manifest-path (Join-Path $projectRoot 'app\src-tauri\Cargo.toml') `
    --target $Target --release --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Force -Path $Output | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Output 'runtime\tools') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Output 'docker') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Output 'docs') | Out-Null
Copy-Item -LiteralPath (Join-Path $appBuildDirectory "$Target\release\xcaliber-desktop.exe") `
    -Destination (Join-Path $Output 'Xcaliber.exe') -Force
Copy-Item -LiteralPath (Join-Path $cliOutput 'xcaliber.exe') `
    -Destination (Join-Path $Output 'runtime\xcaliber.exe') -Force
Copy-Item -LiteralPath (Join-Path $cliOutput 'xcaliber-engine.exe') `
    -Destination (Join-Path $Output 'runtime\xcaliber-engine.exe') -Force
Copy-Item -LiteralPath (Join-Path $cliOutput 'tools\pack_trunk.py') `
    -Destination (Join-Path $Output 'runtime\tools\pack_trunk.py') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'docker\compose.release.yaml') `
    -Destination (Join-Path $Output 'docker\compose.yaml') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'README.md') -Destination $Output -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination $Output -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'NOTICE') -Destination $Output -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'VALIDATION.md') `
    -Destination (Join-Path $Output 'docs\VALIDATION.md') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'RELEASE-MANIFEST.txt') `
    -Destination (Join-Path $Output 'RELEASE-MANIFEST.txt') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'CHECKSUMS.sha256') `
    -Destination (Join-Path $Output 'CHECKSUMS.sha256') -Force

Write-Output "Windows desktop and CLI staged at $Output"
