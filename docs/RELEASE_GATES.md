# Release gates

## Executable on every source release

- Rust formatting, workspace check, Clippy with warnings denied, unit/integration tests,
  and doctests.
- CLI shard-manifest total and revision-parser tests.
- Weightless K3 model oracle, tensor binding, tokenizer, cache, MXFP4, KDA, MLA, router,
  and safetensors tests.
- Local-only source audit and Docker/Compose shape audit.
- Tauri MSVC-target check, Clippy, command-allowlist tests, frontend syntax check, and
  portable executable smoke launch.
- Windows CLI smoke tests for help, requirements, JSON doctor output, insufficient-space
  refusal, and invalid-model refusal.

## Required before a full-model claim

- Exact 96-shard checkpoint checksum verification.
- Real checkpoint metadata bind.
- Real two-layer tests.
- Full 93-layer generation using official weights.
- Multi-turn XTML chat through Docker/API.
- Failure injection for missing/truncated shards and interrupted reads.

## Required before a GPU claim

- Vulkan initialization on the named GPU and driver.
- GPU/CPU numerical comparison at documented tolerances.
- Device-loss and out-of-memory fallback tests.
- End-to-end throughput and memory evidence, not only kernel tests.

## Required before a multi-PC K3 claim

- K3-specific expert/layer placement and worker protocol.
- Routing and logits parity against one-machine exact execution.
- Worker loss, retry, timeout, and split-brain tests.
- Measured LAN bandwidth/latency and end-to-end token throughput.
