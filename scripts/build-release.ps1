param(
    [string]$Cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe",
    [string]$Output = "",
    [string]$PortableAppTarget = "x86_64-pc-windows-msvc",
    [switch]$SkipInstaller
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
if (-not $Output) {
    $Output = Join-Path $projectRoot 'dist\release'
}
$version = '1.3.0'
New-Item -ItemType Directory -Force -Path $Output | Out-Null

& (Join-Path $PSScriptRoot 'audit.ps1') -ProjectRoot $projectRoot
if (-not $?) { throw 'Release audit failed' }

$cliOutput = Join-Path $projectRoot 'dist\windows-cli'
& (Join-Path $PSScriptRoot 'build-windows.ps1') -Cargo $Cargo -Output $cliOutput
if (-not $?) { throw 'Windows CLI staging failed' }

$cli = Join-Path $cliOutput 'xcaliber.exe'
$engine = Join-Path $cliOutput 'xcaliber-engine.exe'
& $cli --version
if ($LASTEXITCODE -ne 0) { throw 'CLI version smoke test failed' }
& $cli requirements | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'CLI requirements smoke test failed' }
& $engine --help | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'exact engine help smoke test failed' }
$doctorJson = & $cli doctor --json
$doctorExit = $LASTEXITCODE
if ($doctorExit -notin 0, 2) { throw "CLI doctor failed with exit code $doctorExit" }
$doctor = $doctorJson | ConvertFrom-Json
if ($doctor.product -ne 'xcaliber' -or $doctor.version -ne $version -or -not $doctor.local_only) {
    throw 'CLI doctor returned invalid release identity'
}
& $cli chat --api-url http://example.com:8000 --prompt refusal-test 2>$null
if ($LASTEXITCODE -eq 0) { throw 'CLI chat accepted a non-loopback endpoint' }

$cliArchive = Join-Path $Output "Xcaliber-$version-windows-x64-cli.zip"
Compress-Archive -Path (Join-Path $cliOutput '*') -DestinationPath $cliArchive -CompressionLevel Optimal -Force

$appOutput = Join-Path $projectRoot 'dist\windows-app'
& (Join-Path $PSScriptRoot 'build-app.ps1') -Cargo $Cargo -Output $appOutput -Target $PortableAppTarget
if (-not $?) { throw 'Windows Studio staging failed' }
$appArchive = Join-Path $Output "Xcaliber-$version-windows-x64-studio.zip"
Compress-Archive -Path (Join-Path $appOutput '*') -DestinationPath $appArchive -CompressionLevel Optimal -Force

if (-not $SkipInstaller) {
    & (Join-Path $PSScriptRoot 'build-installer.ps1') -Cargo $Cargo -Output $Output
    if (-not $?) { throw 'Windows installer staging failed' }
}

$artifacts = Get-ChildItem -LiteralPath $Output -File | Where-Object {
    $_.Name -like "Xcaliber-$version-*" -or $_.Name -like "Xcaliber_${version}_*"
}
$checksums = foreach ($artifact in $artifacts) {
    $hash = (Get-FileHash -LiteralPath $artifact.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $($artifact.Name)"
}
Set-Content -LiteralPath (Join-Path $Output 'SHA256SUMS.txt') -Value $checksums -Encoding utf8NoBOM
Write-Output "Release staged at $Output"
