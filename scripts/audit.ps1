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
$releaseCompose = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'docker\compose.release.yaml')
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
Assert-True ($releaseCompose.Contains('ghcr.io/karvifi/xcaliber:1.3.0')) "release Compose does not use the versioned public image"
Assert-True (-not $releaseCompose.Contains('build:')) "release Compose unexpectedly requires a source build"
Assert-True ($releaseCompose.Contains('127.0.0.1:${K3_PORT:-8000}:8000')) "release Compose is not host-loopback only"
Assert-True ($releaseCompose.Contains('COLI_API_KEY')) "release Compose has no local authentication"
Assert-True ($releaseCompose.Contains('K3_TOPP: "${K3_TOPP:-0}"')) "release Compose changes expert selection by default"
Assert-True ((Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot '.dockerignore')).Contains('!runtime/colibri/c/**')) "Docker context allowlist is missing the C runtime"

$ownedDirectories = @(
    (Join-Path $ProjectRoot 'cli'),
    (Join-Path $ProjectRoot 'docker'),
    (Join-Path $ProjectRoot 'app\ui'),
    (Join-Path $ProjectRoot 'app\src-tauri\src')
)
$ownedFiles = $ownedDirectories | ForEach-Object {
    Get-ChildItem -LiteralPath $_ -File -Recurse
}
$ownedFiles += Get-Item -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\tauri.conf.json')
$ownedFiles += Get-Item -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\tauri.release.conf.json')
$hostedPattern = 'api\.moonshot|platform\.moonshot|api\.kimi|KIMI_API_KEY|MOONSHOT_API_KEY'
$hostedHits = $ownedFiles | Select-String -Pattern $hostedPattern
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
$tauriReleaseConfig = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\tauri.release.conf.json')
$capabilities = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\capabilities\default.json')
Assert-True ($tauri.Contains('build_docker_arguments')) "Desktop has no fixed Docker operation builder"
Assert-True ($tauri.Contains('operation is not allowed')) "Desktop command allowlist is missing"
Assert-True (-not $tauri.Contains('cmd.exe')) "Desktop contains a command-shell bridge"
Assert-True ($tauriConfig.Contains("connect-src 'self' ipc: http://ipc.localhost http://127.0.0.1:* http://localhost:*")) "Desktop CSP is not localhost-only"
Assert-True ($capabilities.Contains('core:default')) "Desktop default capability is missing"
Assert-True (-not $capabilities.Contains('shell:')) "Desktop grants a shell capability"
Assert-True ($tauri.Contains('BaseDirectory::Resource')) "installed desktop does not resolve bundled resources"
Assert-True ($tauri.Contains('runtime/xcaliber.exe')) "portable desktop does not use the collision-safe CLI location"
Assert-True (-not $tauri.Contains('&["xcaliber.exe", "../windows-cli/xcaliber.exe"]')) "desktop can confuse Xcaliber.exe with xcaliber.exe on Windows"
Assert-True ($tauriReleaseConfig.Contains('"active": true')) "release installer bundling is not active"
Assert-True ($tauriReleaseConfig.Contains('"targets": ["nsis"]')) "release installer is not NSIS"
Assert-True ($tauriReleaseConfig.Contains('xcaliber.exe')) "release installer does not include the CLI"
Assert-True ($tauriReleaseConfig.Contains('compose.release.yaml')) "release installer does not include release Compose"
$portableBuild = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'scripts\build-app.ps1')
Assert-True ($portableBuild.Contains("runtime\xcaliber.exe")) "portable app builder does not isolate the CLI under runtime"
Assert-True (-not $portableBuild.Contains('Get-ChildItem -LiteralPath $cliOutput | Copy-Item')) "portable app builder still has the case-insensitive executable collision"
Assert-True ($portableBuild.Contains("RELEASE-MANIFEST.txt")) "portable app builder omits the release manifest"
Assert-True ($portableBuild.Contains("CHECKSUMS.sha256")) "portable app builder omits the source checksum manifest"

$ui = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\ui\index.html')
$uiScript = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\ui\app.js')
$uiCore = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'app\ui\core.mjs')
Assert-True ($ui.Contains('assets/xcaliber-mark.svg')) "desktop does not use the Xcaliber mark"
Assert-True (-not $ui.Contains('K3W Local')) "desktop still contains the internal K3W brand"
foreach ($view in @('overview', 'models', 'playground', 'runtime', 'monitor', 'activity', 'exports', 'settings')) {
    Assert-True ($ui.Contains("data-view=`"$view`"")) "desktop studio navigation is missing: $view"
    Assert-True ($ui.Contains("id=`"$view`"")) "desktop studio view is missing: $view"
}
Assert-True ($uiCore.Contains('host === "localhost" || host === "127.0.0.1"')) "desktop profile policy is not limited to CSP-compatible loopback hosts"
Assert-True ($uiCore.Contains('url.username || url.password')) "desktop profile policy accepts URL credentials"
Assert-True ($uiScript.Contains('redirect: "error"')) "desktop local API requests can follow redirects"
Assert-True (-not $uiScript.Contains('window.alert')) "desktop uses blocking browser alert dialogs"
Assert-True (-not $uiScript.Contains('window.confirm')) "desktop uses blocking browser confirmation dialogs"
Assert-True (-not ($ui + $uiScript + $uiCore).ToLowerInvariant().Contains('unsloth')) "desktop UI contains reference-product identifiers"
Assert-True (Test-Path -LiteralPath (Join-Path $ProjectRoot 'app\tests\core.test.mjs') -PathType Leaf) "desktop frontend tests are missing"
Assert-True (Test-Path -LiteralPath (Join-Path $ProjectRoot 'docs\CLEAN-ROOM-STUDIO-MAP.md') -PathType Leaf) "clean-room desktop feature map is missing"

$referencedUiIds = [regex]::Matches($uiScript, '\$\("([^"]+)"\)') | ForEach-Object { $_.Groups[1].Value } | Sort-Object -Unique
$declaredUiIds = [regex]::Matches($ui, 'id="([^"]+)"') | ForEach-Object { $_.Groups[1].Value }
$missingUiIds = @($referencedUiIds | Where-Object { $_ -notin $declaredUiIds })
$duplicateUiIds = @($declaredUiIds | Group-Object | Where-Object Count -gt 1)
Assert-True ($missingUiIds.Count -eq 0) "desktop script references missing element ids: $($missingUiIds -join ', ')"
Assert-True ($duplicateUiIds.Count -eq 0) "desktop markup contains duplicate element ids: $(($duplicateUiIds.Name) -join ', ')"
foreach ($asset in @(
    'assets\brand\xcaliber-mark.svg',
    'assets\brand\xcaliber-mark.png',
    'assets\brand\xcaliber-wordmark.svg',
    'assets\brand\xcaliber-readme.svg',
    'assets\brand\xcaliber-compute-field.png',
    'assets\screenshots\xcaliber-overview.png',
    'assets\screenshots\xcaliber-studio-overview.png',
    'app\src-tauri\icons\icon.ico',
    'app\src-tauri\icons\icon.png'
)) {
    $assetPath = Join-Path $ProjectRoot $asset
    Assert-True (Test-Path -LiteralPath $assetPath -PathType Leaf) "brand asset is missing: $asset"
    Assert-True ((Get-Item -LiteralPath $assetPath).Length -gt 0) "brand asset is empty: $asset"
}
$appIconHash = (Get-FileHash -LiteralPath (Join-Path $ProjectRoot 'app\src-tauri\icons\icon.ico') -Algorithm SHA256).Hash
$upstreamIconHash = (Get-FileHash -LiteralPath (Join-Path $ProjectRoot 'runtime\colibri\desktop\src-tauri\icons\icon.ico') -Algorithm SHA256).Hash
Assert-True ($appIconHash -ne $upstreamIconHash) "desktop still uses the bundled Colibri icon"

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
Assert-True ($cliManifest.Contains('version = "1.3.0"')) "CLI release version is not 1.3.0"
Assert-True ($appManifest.Contains('version = "1.3.0"')) "desktop release version is not 1.3.0"
Assert-True ($tauriConfig.Contains('"version": "1.3.0"')) "Tauri release version is not 1.3.0"

$localApi = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'cli\src\local_api.rs')
Assert-True ($localApi.Contains('address.ip().is_loopback()')) "CLI local API client does not verify the resolved address"
Assert-True ($localApi.Contains('127.0.0.1') -and $localApi.Contains('localhost')) "CLI local API allowlist is missing"
Assert-True ($localApi.Contains('.strip_prefix("http://")')) "CLI local API client does not require plain loopback HTTP"
Assert-True ($localApi.Contains('endpoint("http://example.com:8000").is_err()')) "CLI local API external-host refusal test is missing"

foreach ($publicFile in @(
    '.github\workflows\release.yml',
    '.github\workflows\container.yml',
    '.github\dependabot.yml',
    '.github\CODEOWNERS',
    '.github\ISSUE_TEMPLATE\bug_report.yml',
    '.github\ISSUE_TEMPLATE\feature_request.yml',
    '.github\pull_request_template.md',
    'CHANGELOG.md',
    'SUPPORT.md',
    'CODE_OF_CONDUCT.md',
    'docs\PRODUCT-READINESS.md'
)) {
    Assert-True (Test-Path -LiteralPath (Join-Path $ProjectRoot $publicFile) -PathType Leaf) "public repository file is missing: $publicFile"
}

$markdownFiles = @(
    Get-ChildItem -LiteralPath $ProjectRoot -File -Filter '*.md'
    Get-ChildItem -LiteralPath (Join-Path $ProjectRoot 'docs') -File -Filter '*.md' -Recurse
    Get-Item -LiteralPath (Join-Path $ProjectRoot 'app\README.md')
    Get-ChildItem -LiteralPath (Join-Path $ProjectRoot 'assets') -File -Filter '*.md' -Recurse
)
$relativeLinkPattern = '(?<!\\)\[[^\]]*\]\((?!https?://|mailto:|#)(?<path>[^)#]+)(?:#[^)]*)?\)'
foreach ($markdown in $markdownFiles) {
    $text = Get-Content -Raw -LiteralPath $markdown.FullName
    foreach ($match in [regex]::Matches($text, $relativeLinkPattern)) {
        $relative = [Uri]::UnescapeDataString($match.Groups['path'].Value.Trim('<', '>'))
        $target = Join-Path $markdown.DirectoryName $relative
        Assert-True (Test-Path -LiteralPath $target) "broken relative Markdown link in $($markdown.FullName): $relative"
    }
}

$ffi = Get-Content -Raw -LiteralPath (Join-Path $ProjectRoot 'cli\src\main.rs')
Assert-True ($ffi.Contains('extern "system"')) "Windows APIs do not use the system ABI"
Assert-True ($ffi.Contains('GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32')) "GlobalMemoryStatusEx signature changed"
Assert-True ($ffi.Contains('path: *const u16')) "GetDiskFreeSpaceExW path is not a wide pointer"

function Get-SourceFiles([string]$Directory) {
    foreach ($item in Get-ChildItem -LiteralPath $Directory -Force) {
        if ($item.PSIsContainer) {
            if ($item.Name -notin @('.git', 'build', 'dist', 'target', '__pycache__', '.pytest_cache')) {
                Get-SourceFiles $item.FullName
            }
        }
        elseif ($item.Extension -ne '.pyc') {
            $item
        }
    }
}
$largeFiles = Get-SourceFiles $ProjectRoot | Where-Object { $_.Length -gt 100MB }
Assert-True (-not $largeFiles) "unexpected file over 100 MB; model weights must not be bundled"

Write-Output 'AUDIT: PASS'
Write-Output "  shard manifest: 96 files / $total bytes"
Write-Output '  Docker target: Kimi K3'
Write-Output '  Docker resources: auto-tier, direct I/O, exact expert selection default'
Write-Output '  release container: GHCR image + no source-build requirement'
Write-Output '  local service: loopback publish + local password'
Write-Output '  desktop: typed allowlist + localhost CSP + no shell capability'
Write-Output "  desktop studio: 8 views + $($referencedUiIds.Count) bound element ids + redirect-safe loopback profiles"
Write-Output '  installer: NSIS + bundled CLI/runtime resources'
Write-Output '  brand: original mark, app icon, banner, and artwork'
Write-Output '  local chat CLI: resolved loopback address enforcement'
Write-Output '  public repository: release workflows, issue forms, ownership, and support files'
Write-Output "  owned Markdown links: $($markdownFiles.Count) files checked"
Write-Output '  Cargo roots/targets/locks: consistent'
Write-Output '  Win32 FFI shape: system ABI + wide path'
Write-Output '  hosted Kimi API references in control code: 0'
Write-Output '  todo!/unimplemented!: 0'
Write-Output '  bundled files over 100 MB: 0'
