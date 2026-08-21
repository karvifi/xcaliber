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
function Get-ManifestFiles([string]$Directory) {
    foreach ($item in Get-ChildItem -LiteralPath $Directory -Force) {
        if ($item.PSIsContainer) {
            if ($item.Name -notin $excludedDirectories) {
                Get-ManifestFiles $item.FullName
            }
        }
        elseif ($item.FullName -ne $manifest -and $item.Extension -ne '.pyc') {
            $item
        }
    }
}
$files = @(Get-ManifestFiles $ProjectRoot) | Sort-Object FullName

$lines = foreach ($file in $files) {
    $relative = [IO.Path]::GetRelativePath($ProjectRoot, $file.FullName).Replace('\', '/')
    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $relative"
}

Set-Content -LiteralPath $manifest -Value $lines -Encoding utf8NoBOM
Write-Output "CHECKSUMS: GENERATED - $($files.Count) files"
