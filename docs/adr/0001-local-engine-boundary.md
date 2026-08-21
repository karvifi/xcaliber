# ADR-0001: Use local engines behind one control CLI

## Status

Accepted

## Context

The product must run Kimi K3 locally, work without a GPU, support Docker, and later gain
a Tauri interface. The full checkpoint is too large to bundle. The Rust reference has
the best native Windows/CPU portability; Colibri has the stronger persistent chat, API,
storage pipeline, and Vulkan path.

## Decision

Use `xcaliber` as a small local control plane. Package the Rust engine for native Windows and
build the Colibri Kimi engine in Docker. Both consume the same user-owned checkpoint.
The Tauri application calls the local CLI/service rather than implementing model
math in the UI process.

## Consequences

### Positive

- CPU-only execution is always available.
- Docker and desktop can share one local API contract.
- Model files remain external and replaceable.
- Numerical engine tests remain independent of UI changes.

### Negative

- Native and container paths currently have different feature depth.
- The low-memory Rust path needs an extra packed trunk.
- Full release certification still requires access to the complete checkpoint.

## Alternatives considered

- Hosted Moonshot API: rejected because it violates the local-only requirement.
- Copy Unsloth Studio: rejected because its broad training stack is not the K3 inference
  engine required here and Xcaliber needs an independently reviewable implementation.
- Use only Colibri: rejected because the exact native Rust path provides a distinct
  numerical contract and does not require Docker.
