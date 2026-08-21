# Product readiness

This page separates software that is ready to use from outcomes that require assets or
hardware the repository cannot contain.

## Ready from the source release

| Surface | Ready behavior | Required from the user |
|---|---|---|
| Windows CLI | Hardware plan, doctor, guarded model pull, checksum verification, trunk packing, exact launch, and loopback local chat | Rust only when building from source; model files only for model execution |
| Portable CLI archive | CLI, exact engine, trunk tool, release Compose file, licenses, and validation record | Windows x64 |
| Desktop controller | First-run plan, fixed CLI actions, fixed Docker actions, and loopback chat | Windows, WebView2, bundled CLI resources |
| Docker source | Reproducible K3 engine build, non-root container, loopback publish, authentication, and health check | Docker and the complete checkpoint |
| Public container workflow | Builds and publishes versioned GHCR images with provenance | A successful GitHub-hosted runner |

## Usable modes

### Smaller local model

This is the practical interactive path for many 16–32 GB computers. Start a separately
licensed OpenAI-compatible local server, then use either:

```powershell
xcaliber.exe chat --api-url http://127.0.0.1:8000 --model local-model --prompt "Hello"
```

or the desktop Local chat page. This mode is local and useful, but it is not official
Kimi K3.

### Exact Kimi K3

The exact Rust engine is implemented and covered by weightless tensor, tokenizer,
binding, cache, operator, and model-oracle tests. Running it requires the complete
official 1.56 TB checkpoint and additional packed-trunk storage. Ordinary SSDs are
expected to be slow because exact execution must move enormous amounts of model data.

### Adaptive Kimi K3

The Docker/Colibri path can quantize the resident dense path and tier experts across
RAM, SSD, and optional Vulkan memory. It is intended to improve resource use, but it
does not preserve bit-identical official precision.

## External release gates

These items cannot be completed by source edits alone and are not claimed:

- access to the complete official checkpoint and its model-license acceptance;
- a full 96-shard checksum verification and 93-layer generated response;
- named GPU initialization, CPU/GPU numerical parity, OOM, and device-loss tests;
- performance measurements on each advertised hardware configuration;
- an organization code-signing certificate and signed Windows installer;
- successful GitHub-hosted runner allocation for public binary/container publication;
- an implemented and parity-tested multi-computer K3 worker protocol.

The detailed gate definitions are in [RELEASE_GATES.md](RELEASE_GATES.md), and actual
executed evidence is in [../VALIDATION.md](../VALIDATION.md).
