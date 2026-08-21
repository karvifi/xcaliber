param(
    [string]$ProjectRoot = ""
)

$ErrorActionPreference = 'Stop'
if (-not $ProjectRoot) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$ProjectRoot = (Resolve-Path -LiteralPath $ProjectRoot).Path
$manifest = Join-Path $ProjectRoot 'CHECKSUMS.sha256'
if (-not (Test-Path -LiteralPath $manifest -PathType Leaf)) {
    throw "checksum manifest not found: $manifest"
}

$checked = 0
foreach ($line in Get-Content -LiteralPath $manifest) {
    if (-not $line) { continue }
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') {
        throw "invalid checksum line: $line"
    }
    $expected = $Matches[1]
    $relative = $Matches[2].Replace('/', [IO.Path]::DirectorySeparatorChar)
    $file = Join-Path $ProjectRoot $relative
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "missing checksum target: $relative"
    }
    $actual = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        throw "checksum mismatch: $relative"
    }
    $checked++
}

Write-Output "CHECKSUMS: PASS - $checked files"
