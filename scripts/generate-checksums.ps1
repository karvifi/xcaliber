param(
    [string]$ProjectRoot = "",
    [switch]$IncludeBuildArtifacts
)

$ErrorActionPreference = 'Stop'
if (-not $ProjectRoot) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$ProjectRoot = (Resolve-Path -LiteralPath $ProjectRoot).Path
$manifest = Join-Path $ProjectRoot 'CHECKSUMS.sha256'

$excludedDirectories = if ($IncludeBuildArtifacts) {
    @('.git', '__pycache__', '.pytest_cache')
} else {
    @('.git', 'build', 'dist', 'target', '__pycache__', '.pytest_cache')
}
$files = Get-ChildItem -LiteralPath $ProjectRoot -File -Recurse | Where-Object {
    if ($_.FullName -eq $manifest -or $_.Extension -eq '.pyc') { return $false }
    $relative = [IO.Path]::GetRelativePath($ProjectRoot, $_.FullName)
    $parts = $relative -split '[\\/]'
    -not ($parts | Where-Object { $_ -in $excludedDirectories })
} | Sort-Object FullName

$lines = foreach ($file in $files) {
    $relative = [IO.Path]::GetRelativePath($ProjectRoot, $file.FullName).Replace('\', '/')
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $relative"
}

Set-Content -LiteralPath $manifest -Value $lines -Encoding utf8NoBOM
Write-Output "CHECKSUMS: GENERATED - $($files.Count) files"
