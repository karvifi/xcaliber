# Validation record

Release: Xcaliber 1.2.0
Validation date: 2026-08-21
Host: Windows, Intel Core i7-9750H (6 cores/12 threads), 34.2 GB RAM,
Intel UHD 630, NVIDIA GeForce GTX 1660 Ti (6 GB)

Toolchain used:

- rustc/cargo 1.94.1, Windows GNU target for the executed local release build;
- Node.js 24.14.1;
- PowerShell 7;
- Tauri 2.11.5;
- actionlint 1.7.12, downloaded from its official GitHub release and verified against
  GitHub's published SHA-256 digest.

## Executed and passed for 1.2.0

### Core Rust workspace

- Formatting check passed.
- Locked/offline workspace check passed for all targets.
- Locked/offline Clippy passed for all targets with warnings denied.
- Locked/offline release tests passed:
  - 63 tests passed;
  - 2 real-checkpoint tests were ignored by their explicit 1.56 TB checkpoint guard;
  - 0 tests failed.
- The new loopback chat parser and chunked-HTTP decoder tests passed.

### Portable Windows CLI

- `xcaliber 1.2.0` GNU release executable built.
- Version, requirements, doctor JSON, and exact-engine help smoke tests passed.
- Doctor returned the intentional not-ready exit status on this host.
- A local TCP mock returned an OpenAI-compatible response through
  `xcaliber chat`; the command printed `local smoke passed`.
- A non-loopback URL was refused before a connection attempt.
- The portable archive contains the CLI, exact engine, packing tool, release Compose
  file, brand mark, licenses, release manifest, README, and validation record.

Portable archive:

- File: `Xcaliber-1.2.0-windows-x64-cli.zip`
- Size before the final evidence/checksum regeneration: 1,038,807 bytes
- The final published SHA-256 is generated after this validation file and all source
  checks are complete; see the attached `SHA256SUMS.txt`.

### Desktop and installer source

- The isolated Tauri workspace passed Windows GNU `cargo check`.
- Clippy passed for all desktop targets with warnings denied.
- Both desktop test executables linked.
- A clean-target compiler check with the release configuration passed, including
  validation and copying of every installer resource.
- That release-config check found and fixed duplicate Tauri resource destinations
  before publication.
- JavaScript syntax and both Tauri JSON configurations passed parsing.

The linked GNU test executable cannot start on this Windows installation and returns
`STATUS_ENTRYPOINT_NOT_FOUND`. This is the same MinGW/UCRT loader limitation
recorded for 1.1.0. The four command-boundary tests were not counted as executed for
1.2.0. The repository release workflow runs them with the supported MSVC target before
building NSIS.

### Release, security, and repository gates

- `scripts/audit.ps1` passed:
  - exact 96-shard table totaling 1,560,936,091,448 bytes;
  - Kimi-only Docker target and automatic resource tiering;
  - separate source-build and GHCR release Compose definitions;
  - loopback publish, local password, direct I/O, and exact expert-selection defaults;
  - typed desktop command allowlist, localhost CSP, and no shell capability;
  - NSIS release configuration and bundled runtime resources;
  - original logo, app icon, banner, and brand artwork;
  - CLI resolved-loopback enforcement;
  - Cargo target, version, lockfile, license, Win32 ABI, and wide-path shape;
  - public release workflows, issue forms, ownership, support, and conduct files;
  - no hosted Kimi credential references, unfinished Rust macros, or files over 100 MB.
- Every PowerShell release script passed parser validation.
- All GitHub Actions workflows passed actionlint 1.7.12.
- Offline cargo-audit scanned 1,186 RustSec advisories:
  - no vulnerability was reported for the 20-package core lockfile;
  - no vulnerability was reported for the desktop lockfile;
  - 17 allowed warnings describe unmaintained/unsound cross-platform GTK3/UNIC
    transitive packages present in Tauri's lockfile. Those packages are not compiled
    for this Windows-only desktop release, but the warnings remain visible.

## Not executed and not claimed

- The official Kimi K3 checkpoint was not downloaded or present.
- No complete 96-shard checksum verification or 93-layer generated response was run.
- Docker is not installed on this host, so the image and Compose service were not
  built or started locally.
- No GHCR image digest exists until the public container workflow succeeds.
- No Vulkan/GPU execution, numerical parity, OOM, device-loss, throughput, or hardware
  certification was run.
- No NSIS installer was produced locally because this host lacks the MSVC C++ tools and
  Tauri CLI. Installer production is gated by the tagged Windows workflow and is not
  counted as passed until that job attaches the file.
- Executables are not signed; the project has no protected code-signing certificate.
- The GitHub-hosted workflows are not counted as passed until GitHub assigns runners
  and completes them.
- The optional multi-computer exact-K3 worker protocol is not implemented.

## Result for this computer

The primary portable CLI is built and usable for machine planning, guarded model
management, the exact-engine launcher, and loopback chat with a separately running
local OpenAI-compatible model.

This computer does not have enough storage for the 1.561 TB official checkpoint,
Docker is absent, and 34.2 GB RAM plus a 6 GB GPU cannot hold full K3. The correct
interactive route here is a smaller separately licensed local model. Xcaliber exposes
that route without renaming it as Kimi K3.
