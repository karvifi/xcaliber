# Xcaliber

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)

Xcaliber is a public, local-first control plane and inference workspace for running
very large sparse language models—currently focused on
[Moonshot AI's Kimi K3](https://github.com/MoonshotAI/Kimi-K3)—across imperfect
consumer hardware. It includes a Rust CLI, an exact streamed CPU engine, an adaptive
Docker/Colibri runtime, and an original Tauri desktop controller.

Xcaliber uses model files on your own storage. It does **not** call Moonshot's hosted
inference API, does not require a Kimi API key, and does not upload prompts. Model
weights are not included in this repository.

## Read this before downloading Kimi K3

Kimi K3 is not a normal laptop-sized model. The official architecture has roughly
2.8 trillion total parameters, about 104 billion active parameters per token, 93
layers, and 896 routed experts. The published 96-shard checkpoint represented by
Xcaliber's pinned manifest totals exactly **1,560,936,091,448 bytes**. Exact low-memory
execution also needs roughly 109 GB for a packed dense trunk.

Sparse activation lowers arithmetic; it does not make the checkpoint small. Xcaliber's
8 GiB reference evidence completed all 93 layers but measured 72.4722 seconds per
token. A 32 GB laptop with a 6 GB GPU cannot turn that storage traffic into fast,
interactive full-K3 chat. Xcaliber reports this before beginning a download instead of
hiding it behind a progress screen.

Run the planner first:

```powershell
xcaliber.exe plan --model-dir E:\Models\Kimi-K3 --measure-io
```

The planner detects CPU threads, available RAM, NVIDIA VRAM, free model-drive space,
Docker availability, and optional measured sequential-read performance. Its nonzero
"not ready" exit status is intentional when requirements are missing.

## Project status

| Component | Status | What that means |
|---|---|---|
| Rust CLI | Implemented and tested | Plans hardware, verifies requirements, pulls pinned shards, packs the trunk, and launches exact inference |
| Exact streamed engine | Implemented and weightless-tested | Preserves the official tensor path but is storage-bandwidth bound |
| Adaptive Docker engine | Source and configuration complete | Quantized dense path plus RAM/SSD/Vulkan expert tiers; not numerically identical to the exact path |
| Tauri desktop app | Implemented and Windows smoke-tested | Fixed allowlisted CLI/Docker operations and localhost chat; no arbitrary shell bridge |
| CPU-only operation | Supported | Exact Rust and C engines build without a GPU |
| Optional GPU tier | Wired through Colibri Vulkan settings | Not certified with real K3 weights in this release |
| Multi-computer K3 pool | Research/design only | K3-specific workers, parity, recovery, and failover are not implemented |
| Official checkpoint run | Not claimed | The validation host did not contain the 1.56 TB checkpoint |

See [VALIDATION.md](VALIDATION.md) for executed evidence and
[RELEASE_GATES.md](docs/RELEASE_GATES.md) for gates that remain hardware- or
checkpoint-dependent.

## Execution modes

Xcaliber keeps four fundamentally different outcomes explicit:

1. **Exact streamed K3** preserves the official model semantics and operates with low
   RAM by reading tensors and experts from storage. It is usable for correctness and
   experimentation, not expected to be interactive on an ordinary SSD.
2. **Adaptive local K3** keeps a quantized dense path resident and tiers routed experts
   across RAM, SSD, and optional Vulkan memory. It can reduce traffic substantially,
   but quantization means it is not bit-identical to the exact official path.
3. **Smaller surrogate model** is the practical interactive choice for many 16–32 GB
   systems. Xcaliber never labels a distilled or smaller model as official Kimi K3.
4. **Heterogeneous cluster** could aggregate several ordinary systems, but the exact
   K3 worker protocol and distributed correctness tests remain future work.

The research and bandwidth calculations behind these choices are in
[docs/RESEARCH-AND-PERFORMANCE.md](docs/RESEARCH-AND-PERFORMANCE.md).

## Get the source

```bash
git clone https://github.com/karvifi/xcaliber.git
cd xcaliber
```

Do not clone model weights into this Git repository. Put them on a dedicated fast
drive and pass that location to Xcaliber.

## Windows CLI

### Build

Install stable Rust. On Windows, install the MSVC Rust target and Microsoft C++ Build
Tools, then run:

```powershell
.\scripts\build-windows.ps1
```

The portable CLI folder is produced under `dist/windows-cli` and contains
`xcaliber.exe`, `xcaliber-engine.exe`, the trunk-packing tool, notices, and validation
information.

A normal developer build is:

```powershell
cargo build --workspace --release --locked
```

### Inspect the machine and model

```powershell
xcaliber.exe --version
xcaliber.exe requirements
xcaliber.exe plan --model-dir E:\Models\Kimi-K3 --measure-io
xcaliber.exe doctor --model-dir E:\Models\Kimi-K3 --json
```

### Pull the official checkpoint

```powershell
xcaliber.exe pull --destination E:\Models\Kimi-K3
```

Pulls are resumable, pinned to an immutable Hugging Face revision, size-checked, and
SHA-256 verified by default. Xcaliber refuses the pull when the selected destination
does not have at least 1.70 TB free. `--skip-checksum` exists for recovery work but is
not recommended.

Moonshot's model repository and model license remain authoritative for access to and
use of the weights. Xcaliber's AGPL license does not relicense the model.

### Pack and run the exact engine

```powershell
xcaliber.exe pack `
  --model-dir E:\Models\Kimi-K3 `
  --destination E:\Models\Kimi-K3-trunk

xcaliber.exe run `
  --model-dir E:\Models\Kimi-K3 `
  --trunk E:\Models\Kimi-K3-trunk `
  --prompt "Explain sparse mixture-of-experts routing." `
  --gen 32
```

Native decode uses incremental state and lossless n-gram draft proposals. Every
accepted draft token is verified by the exact engine. Pass `--spec 0` to disable
drafting. Environment overrides are `XCALIBER_ENGINE` and `XCALIBER_PACK_TOOL`.

## Docker adaptive service

Docker provides the persistent Colibri engine and a local OpenAI-compatible endpoint.
It still requires the complete K3 checkpoint.

```powershell
$env:MODEL_DIR = 'E:\Models\Kimi-K3'
$env:K3_LOCAL_API_KEY = 'replace-this-local-password'
docker compose -f docker\compose.yaml up --build
```

The API is published only on loopback by default:

```text
http://127.0.0.1:8000/v1/chat/completions
```

Example request:

```bash
curl http://127.0.0.1:8000/v1/chat/completions \
  -H "Authorization: Bearer replace-this-local-password" \
  -H "Content-Type: application/json" \
  -d '{"model":"kimi-k3-local","messages":[{"role":"user","content":"Hello"}]}'
```

Important settings:

| Variable | Default | Purpose |
|---|---:|---|
| `K3_PORT` | `8000` | Host loopback port |
| `K3_LOCAL_API_KEY` | `xcaliber-local` | Local gateway password; change it |
| `K3_BITS` | `4` | Adaptive dense-path quantization |
| `K3_MLA_BITS` | `8` | MLA-path quantization |
| `K3_DIRECT` | `1` | Direct storage I/O where supported |
| `K3_PIPE` | `1` | Pipeline loading and execution |
| `K3_LOAD_THREADS` | `4` | Storage-loading workers |
| `K3_VK` | `0` | Enable optional Vulkan tier with `1` |
| `K3_VK_GB` | `0` | Vulkan memory budget; `0` lets the planner decide |
| `K3_TOPP` | `0` | Keep all officially selected routed experts |
| `K3_POLICY` | `quality` | Adaptive resource policy |

Do not change the Compose port binding to `0.0.0.0` unless you understand the
authentication, firewall, and AGPL source-offer implications of providing a network
service.

## Desktop app

The `app` directory contains an original Tauri 2 desktop controller. It has no npm
dependency tree or downloaded web frontend. Its content security policy permits only
the application itself, Tauri IPC, and localhost API connections. The Tauri capability
file grants no shell plugin.

Build on Windows with Rust's MSVC target, Microsoft C++ Build Tools, and WebView2:

```powershell
.\scripts\build-app.ps1
```

Keep the generated `Xcaliber.exe`, `xcaliber.exe`, `xcaliber-engine.exe`, and bundled
`docker` and `runtime` directories together. The app can:

- display the hardware plan and blockers;
- verify, pull, and pack the official checkpoint;
- run an exact one-shot prompt;
- start, stop, inspect, and read logs from the fixed Docker Compose service;
- chat with the localhost OpenAI-compatible endpoint;
- cancel only the process it started.

It cannot execute arbitrary commands. `XCALIBER_CLI` may point it to a developer CLI
build.

## Repository layout

```text
app/                 Tauri desktop controller and static UI
cli/                 Rust planner, doctor, model puller, packer, and launcher
docker/              Local adaptive service image and Compose definition
docs/                Architecture, research, ADRs, and release gates
runtime/rust/         Exact streamed Kimi K3 Rust engine
runtime/colibri/      Adaptive C/Python/optional Vulkan runtime
scripts/              Audits, checksums, and Windows release builders
tools/                Model preparation tools shipped with the CLI
```

## Development and validation

Core checks:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --release --locked
pwsh -File scripts\audit.ps1
node --check app\ui\app.js
```

Desktop tests use their own locked Cargo workspace:

```powershell
cargo check --manifest-path app\src-tauri\Cargo.toml --locked
cargo test --manifest-path app\src-tauri\Cargo.toml --locked
```

`scripts/audit.ps1` verifies the 96-shard byte total, Docker safety defaults, absence
of hosted-Kimi credentials, desktop command boundaries, Cargo target shape, Win32 FFI
shape, licensing metadata, unfinished Rust macros, and accidental large files.

Tests that need the official checkpoint are deliberately ignored unless their model
path guard is satisfied. A green weightless test suite is not a full-model
certification.

## Security and privacy

- No Moonshot or Kimi API credential is requested.
- Docker binds to `127.0.0.1` and requires a local password.
- The desktop app exposes typed operations, not a general shell.
- Pulls use a pinned model revision and published checksums.
- Model weights, prompts, caches, and environment files are excluded from Git.

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## Contributing

Issues and pull requests are welcome, especially for reproducible hardware evidence,
storage scheduling, exactness tests, Vulkan parity, safer model management, and the
future heterogeneous-worker protocol. Read [CONTRIBUTING.md](CONTRIBUTING.md) before
submitting changes.

Do not remove evidence gates or describe an adaptive/surrogate result as official K3.
Performance contributions should include the model revision, prompt/token counts,
hardware, storage, driver versions, and complete commands.

## License

Xcaliber-specific source code is released under the
[GNU Affero General Public License version 3 only](LICENSE). If you modify Xcaliber and
provide it as a network service, the AGPL requires offering the corresponding source to
users of that service.

Bundled and modified upstream inference components retain compatible Apache-2.0
notices; see [NOTICE](NOTICE) and [LICENSES/Apache-2.0.txt](LICENSES/Apache-2.0.txt).
Kimi K3 weights are separate and remain governed by Moonshot AI's model license.

## Acknowledgements

Xcaliber incorporates or adapts work from pinned versions of `kimi-k3-in-rust`,
`kimi-k3-in-c`, and `colibri`, recorded in [NOTICE](NOTICE). Research references and
the reasoning behind the execution modes are collected in
[docs/RESEARCH-AND-PERFORMANCE.md](docs/RESEARCH-AND-PERFORMANCE.md).

Unsloth Studio was studied as a workflow reference only. No Unsloth source, assets,
branding, or interface code is included.
