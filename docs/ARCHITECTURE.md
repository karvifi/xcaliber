# Xcaliber local architecture

## System boundary

```text
 Tauri desktop ─┬─ fixed allowlisted CLI operations ── xcaliber CLI
                ├─ fixed Docker Compose operations ─── Docker/Colibri
                └─ HTTP on 127.0.0.1 only ─────────── local chat API
                                      │
                 ┌────────────────────┴────────────────────┐
                 │                                         │
       exact Rust CPU engine                   adaptive Colibri engine
       bf16 + native MXFP4                     quantized dense + MXFP4
       packed trunk streaming                  RAM/SSD/Vulkan expert tiers
                 └────────────────────┬────────────────────┘
                                      │
                         user-selected model drive
```

The control surfaces never contain model weights. The exact and adaptive engines are
separate because their numerical contracts differ. A plan document records the selected
mode rather than silently changing precision.

## Execution modes

| Mode | Official numerical path | Implemented | Ordinary-laptop result |
|---|---:|---:|---|
| Exact streamed K3 | Yes | Yes | Runs with enough external storage; very slow |
| Adaptive local K3 | No; dense precision changes | Yes | Still needs the full checkpoint and roughly 40 GB RAM |
| Smaller surrogate | No | API can connect; model manager not included | Credible interactive option |
| Multi-PC exact K3 | Intended exact | No | Requires a new K3 worker protocol and validation |

## Failure policy

| Failure | Behavior |
|---|---|
| Insufficient storage | Refuse before network transfer |
| Missing or wrong-size shard | Refuse inference and identify the checkpoint as incomplete |
| Wrong model config/tokenizer | Refuse instead of guessing compatibility |
| GPU unavailable | Keep the CPU path; never report a GPU run |
| Low RAM without packed trunk | Refuse exact native launch with packing instructions |
| Docker absent | Planner and app report it; native CLI remains available |
| Concurrent desktop task | Refuse the second task; cancellation targets only the tracked process tree |
| Checksum failure | Fail the pull and do not mark the model ready |

## Security and privacy

- Service publishing is loopback-only and uses a user-changeable local password.
- Tauri commands are typed and allowlisted; no arbitrary program or shell text is accepted.
- Desktop content security policy permits only packaged assets and localhost connections.
- Model directories are read-only inside Docker.
- Hub downloads resolve an immutable revision and verify checksums by default.

## Non-functional targets

- Report capacity, measured bandwidth, and semantic mode independently.
- Keep release artifacts small by excluding model weights and build caches.
- Do not claim full-model or hardware support without the corresponding release gates.
- Prefer exact lossless improvements before optional quality-changing policies.
