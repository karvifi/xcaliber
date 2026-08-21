param(
    [string]$ProjectRoot = ""
)

$ErrorActionPreference = 'Stop'
if (-not $ProjectRoot) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$shards = Get-Content -LiteralPath (Join-Path $ProjectRoot 'cli\assets\shard_sizes.txt')
$total = [int64]0
foreach ($line in $shards) {
    $fields = $line -split '\s+'
    Assert-True ($fields.Count -eq 2) "bad shard manifest line: $line"
    $total += [int64]$fields[1]
}
Assert-True ($shards.Count -eq 96) "shard manifest must contain 96 entries"
Assert-True ($total -eq 1560936091448) "shard manifest total is wrong: $total"

$dockerfile = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'docker\Dockerfile')
$compose = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'docker\compose.yaml')
Assert-True ($dockerfile.Contains('make -C c kimi_k3')) "Dockerfile does not build the Kimi engine"
Assert-True (-not $dockerfile.Contains('COLI_DOCKER_GLM_ONLY')) "Dockerfile still contains the GLM-only guard"
Assert-True ($compose.Contains('127.0.0.1:${K3_PORT:-8000}:8000')) "Compose is not host-loopback only"
Assert-True ($compose.Contains('COLI_API_KEY')) "Container gateway has no local authentication"
Assert-True ($compose.Contains('kimi-k3-local')) "Compose model id is not Kimi K3"
Assert-True ($compose.Contains('K3_DIRECT: "${K3_DIRECT:-1}"')) "Direct I/O is not enabled by default"
Assert-True ($compose.Contains('K3_TOPP: "${K3_TOPP:-0}"')) "Exact expert selection is not the default"
Assert-True ($compose.Contains('--auto-tier')) "Compose does not use the hardware resource planner"
Assert-True (-not $compose.Contains('K3_EXPERT_GB:')) "Compose still forces a fixed expert cache"
Assert-True (-not $compose.Contains('K3_RAM_GB')) "Compose still forces a fixed RAM budget"

$ownedFiles = @(
    (Join-Path $ProjectRoot 'cli'),
    (Join-Path $ProjectRoot 'docker'),
    (Join-Path $ProjectRoot 'app')
)
$hostedPattern = 'api\.moonshot|platform\.moonshot|api\.kimi|KIMI_API_KEY|MOONSHOT_API_KEY'
$hostedHits = Get-ChildItem -LiteralPath $ownedFiles -File -Recurse | Select-String -Pattern $hostedPattern
Assert-True (-not $hostedHits) "hosted Kimi endpoint or credential found in product control code"

$notice = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'NOTICE')
foreach ($pin in @(
    '32474ab322f7ae6f7bea1266ceb2bea63ac2d54a',
    'ff11dce858a2eb8a781224facdffd33a1fa48d25',
    '33e67a9c004b6e608d1f19dfbdcc20793377f94f'
)) {
    Assert-True ($notice.Contains($pin)) "NOTICE is missing upstream pin $pin"
}

$rustFiles = Get-ChildItem -LiteralPath (Join-Path $ProjectRoot 'cli'), (Join-Path $ProjectRoot 'runtime\rust\src'), (Join-Path $ProjectRoot 'app\src-tauri\src') -File -Recurse -Filter '*.rs'
$unfinished = $rustFiles | Select-String -Pattern '\b(todo!|unimplemented!)\s*\('
Assert-True (-not $unfinished) "unfinished Rust macro found"

$tauri = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\src\lib.rs')
$tauriConfig = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\tauri.conf.json')
$capabilities = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\capabilities\default.json')
Assert-True ($tauri.Contains('build_docker_arguments')) "Desktop has no fixed Docker operation builder"
Assert-True ($tauri.Contains('operation is not allowed')) "Desktop command allowlist is missing"
Assert-True (-not $tauri.Contains('cmd.exe')) "Desktop contains a command-shell bridge"
Assert-True ($tauriConfig.Contains("connect-src 'self' ipc: http://ipc.localhost http://127.0.0.1:* http://localhost:*")) "Desktop CSP is not localhost-only"
Assert-True ($capabilities.Contains('core:default')) "Desktop default capability is missing"
Assert-True (-not $capabilities.Contains('shell:')) "Desktop grants a shell capability"

$rootManifest = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'Cargo.toml')
$cliManifest = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'cli\Cargo.toml')
$appManifest = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\Cargo.toml')
Assert-True ($rootManifest.Contains('members = ["cli", "runtime/rust"]')) "Cargo workspace members changed unexpectedly"
Assert-True ($cliManifest.Contains('name = "xcaliber"')) "CLI binary target is missing"
Assert-True ($cliManifest.Contains('license = "AGPL-3.0-only"')) "CLI AGPL license metadata is missing"
Assert-True ($appManifest.Contains('license = "AGPL-3.0-only"')) "desktop AGPL license metadata is missing"
Assert-True ($appManifest.Contains('[workspace]')) "Desktop is not isolated from the root workspace"
Assert-True (Test-Path -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\Cargo.lock')) "Desktop lockfile is missing"
Assert-True ((Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'LICENSE')).Contains('GNU AFFERO GENERAL PUBLIC LICENSE')) "root AGPL license is missing"
Assert-True (Test-Path -LiteralPath (Join-Path $ProjectRoot 'LICENSES\Apache-2.0.txt')) "bundled Apache license text is missing"

$ffi = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'cli\src\main.rs')
Assert-True ($ffi.Contains('extern "system"')) "Windows APIs do not use the system ABI"
Assert-True ($ffi.Contains('GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32')) "GlobalMemoryStatusEx signature changed"
Assert-True ($ffi.Contains('path: *const u16')) "GetDiskFreeSpaceExW path is not a wide pointer"

$largeFiles = Get-ChildItem -LiteralPath $ProjectRoot -File -Recurse | Where-Object {
    $_.Length -gt 100MB -and $_.FullName -notmatch '[\\/](build|dist|target)[\\/]'
}
Assert-True (-not $largeFiles) "unexpected file over 100 MB; model weights must not be bundled"

Write-Output 'AUDIT: PASS'
Write-Output "  shard manifest: 96 files / $total bytes"
Write-Output '  Docker target: Kimi K3'
Write-Output '  Docker resources: auto-tier, direct I/O, exact expert selection default'
Write-Output '  local service: loopback publish + local password'
Write-Output '  desktop: typed allowlist + localhost CSP + no shell capability'
Write-Output '  Cargo roots/targets/locks: consistent'
Write-Output '  Win32 FFI shape: system ABI + wide path'
Write-Output '  hosted Kimi API references in control code: 0'
Write-Output '  todo!/unimplemented!: 0'
Write-Output '  bundled files over 100 MB: 0'
