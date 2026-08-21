# Validation record

Release: Xcaliber 1.1.0 (engineering release v46)
Validation date: 2026-08-21
Host: Windows, Intel Core i7-9750H (6 cores/12 threads), 34.2 GB RAM,
Intel UHD 630, NVIDIA GeForce GTX 1660 Ti (6 GB)

Toolchain used:

- rustc 1.94.1
- cargo 1.94.1
- Python 3.12.13
- Node.js 24.14.1
- Tauri 2.11.5

## Public Xcaliber tree validation

The final public-name tree was checked again after changing the CLI, desktop package,
Docker service, environment variables, documentation, manifests, and lockfiles from the
internal engineering name to Xcaliber:

- Rust formatting passed.
- Locked offline workspace check passed.
- Locked offline clippy passed with warnings denied.
- Locked offline release tests passed: 60 passed, 2 checkpoint-gated tests ignored.
- The renamed `xcaliber.exe` release binary built and its version/planner smoke tests
  passed; the planner returned the intentional not-ready status on this host.
- The renamed Tauri workspace passed Windows GNU `cargo check` and clippy with warnings
  denied. Its GNU test executable linked but Windows could not start it because the
  MinGW/UCRT runtime reported `STATUS_ENTRYPOINT_NOT_FOUND`.
- A post-rename MSVC cross-check was attempted, but the validation host does not have
  the `clang-cl` C++ frontend required by one Tauri build dependency. The repository CI
  runs the desktop check and four command-boundary tests on GitHub's MSVC-equipped
  Windows runner.

The complete MSVC desktop build, four command-boundary tests, and packaged launch smoke
listed below were executed immediately before the public-name-only rebrand. They are
retained as evidence but are not misrepresented as a post-rename MSVC run.

## Executed and passed

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked --offline`
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- `cargo test --workspace --release --locked --offline`
  - 60 Rust tests passed
  - 2 real-checkpoint tests ignored by their explicit 1.56 TB checkpoint guard
  - 0 failures
- Tauri desktop application, compiled for the Windows MSVC ABI
  - locked dependency check passed
  - clippy with warnings denied passed
  - 4 command-boundary tests passed
  - release executable built and launched for a five-second smoke test
- Selected Colibri Python suite for Kimi architecture, doctor, family registry,
  repacking, CLI usage, resource plans, XTML chat, and the local OpenAI-compatible
  server
  - 278 tests passed
  - 7 tests skipped because they require a separately configured backend or POSIX PTY
  - 0 code failures
  - one 12 GB sparse-fixture test initially exhausted the host's nearly full C: drive;
    its isolated rerun on D: passed, and the fixture was removed
- Windows MinGW build of `runtime/colibri/c/kimi_k3.exe`
  - CPU executable linked successfully
  - `--help` smoke test exited successfully
- Windows CLI smoke tests
  - version and requirements output
  - valid JSON from `doctor` on an empty model directory
  - valid JSON from `plan`, including RAM, storage, Docker, and NVIDIA GPU discovery
  - bundled native engine help
- JavaScript syntax check for the Tauri UI
- PowerShell parse checks for every release script
- `scripts/audit.ps1`
  - exact 96-shard table totaling 1,560,936,091,448 bytes
  - Cargo roots, dependencies, locked targets, and Windows feature coverage
  - Win32 system ABI and wide-path FFI signatures
  - Kimi-only Docker target, automatic tiering, exact expert selection, and direct I/O
  - host-loopback Docker publish and local gateway password
  - typed desktop command allowlist, strict content security policy, and no shell capability
  - no hosted Kimi endpoint or credential in Xcaliber control code
  - no `todo!` or `unimplemented!` in owned/runtime Rust code
  - no bundled model-weight-sized files

## Defect fixed during this pass

The Rust safetensor reader converted normal positive f16 exponents with unsigned
subtraction. Debug builds could panic for valid values with exponents 1 through 14.
The conversion now uses an underflow-safe bias expression, and the previously failing
`tricky_f16_1d` test passes.

## Not executed and not claimed

- The official Kimi K3 checkpoint was not downloaded or present.
- No complete 93-layer generation with real K3 weights was run.
- No full-checkpoint checksum verification was run.
- Docker is not installed on this host, so the image was not built or started.
- No Vulkan/GPU execution or CPU/GPU numerical comparison was run.
- No tokens-per-second, full-model, or hardware certification is claimed.
- The optional multi-computer exact-K3 worker protocol described in the research notes is
  not implemented in this release.

## Result for this computer

The planner measured 34.2 GB of RAM, detected the 6 GB NVIDIA GPU, found no Docker
installation, and found nowhere near the 1.561 TB required for the official checkpoint.
It therefore recommends a smaller local surrogate for interactive work. That mode can
use the desktop app's local OpenAI-compatible connection, but it is not represented as
official Kimi K3.

For official K3, the download guard requires at least 1.70 TB free. Exact streamed
inference is available after that checkpoint is supplied, but it is storage-bandwidth
bound and is not expected to be interactive on ordinary SSDs. The published 8 GiB
reference measurement in this tree took 72.4722 seconds per token; Xcaliber does not hide or
rename that limitation.
