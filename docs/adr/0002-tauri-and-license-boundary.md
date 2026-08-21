# ADR-0002: Build an original K3-only Tauri interface

## Status

Accepted and implemented

## Context

Unsloth Studio demonstrates a useful local workflow and is itself a Tauri application.
Its `studio/*` and `unsloth_cli/*` source is AGPLv3. Xcaliber is independently written,
Kimi-focused, and inference-first rather than a general model-training suite. Xcaliber's
original control-plane code is also published under AGPL-3.0-only; bundled inference
runtimes retain their upstream Apache-2.0 notices.

## Decision

Implement an original Tauri interface over the Xcaliber local service. Reproduce necessary
workflow concepts—drive selection, resumable model pull, model verification, chat,
hardware status, and settings—without copying Unsloth branding, assets, or source.

## Consequences

- Xcaliber's original network-facing application remains AGPL-3.0-only.
- Bundled Apache-2.0 components remain identified in `NOTICE` and `LICENSES`.
- The desktop application remains small because weights and inference stay outside it.
- Pixel-for-pixel Unsloth cloning is not a goal; functional K3 workflow compatibility is.
