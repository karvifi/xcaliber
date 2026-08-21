param(
    [string]$Cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe",
    [string]$Output = ""
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $Output) {
    $Output = Join-Path $projectRoot 'dist\windows-cli'
}
if (-not (Test-Path -LiteralPath $Cargo -PathType Leaf)) {
    throw "cargo was not found at $Cargo"
}

$target = Join-Path $projectRoot 'build\windows-target'
$env:CARGO_TARGET_DIR = $target
& $Cargo build --workspace --bins --release --locked --offline
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

New-Item -ItemType Directory -Force -Path $Output | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Output 'tools') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Output 'docker') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Output 'assets') | Out-Null
Copy-Item -LiteralPath (Join-Path $target 'release\xcaliber.exe') -Destination (Join-Path $Output 'xcaliber.exe') -Force
Copy-Item -LiteralPath (Join-Path $target 'release\k3.exe') -Destination (Join-Path $Output 'xcaliber-engine.exe') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'tools\pack_trunk.py') -Destination (Join-Path $Output 'tools\pack_trunk.py') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'docker\compose.release.yaml') -Destination (Join-Path $Output 'docker\compose.yaml') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'assets\brand\xcaliber-mark.png') -Destination (Join-Path $Output 'assets\xcaliber-mark.png') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'README.md') -Destination (Join-Path $Output 'README.md') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'VALIDATION.md') -Destination (Join-Path $Output 'VALIDATION.md') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'RELEASE-MANIFEST.txt') -Destination (Join-Path $Output 'RELEASE-MANIFEST.txt') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'CHECKSUMS.sha256') -Destination (Join-Path $Output 'CHECKSUMS.sha256') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination (Join-Path $Output 'LICENSE') -Force
Copy-Item -LiteralPath (Join-Path $projectRoot 'NOTICE') -Destination (Join-Path $Output 'NOTICE') -Force

Write-Output "Windows CLI staged at $Output"
