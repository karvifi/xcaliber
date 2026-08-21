# Clean-room desktop studio map

This document records the product-level study used to expand the Xcaliber Windows
application. The local `unsloth-main.zip` reference was inspected as data. No Unsloth source
files, UI markup, styles, images, logos, copy, package dependencies, or identifiers were
copied into Xcaliber.

## Reference inventory

The inspected archive contained 4,441 entries. Its Studio area contained 3,171 files split
across a React frontend, Python backend, and Tauri shell. The public feature areas included
chat, projects, model discovery, loaded models, training, datasets and recipes, RAG, API
monitoring, export, settings, authentication, media generation, updates, diagnostics, and
native desktop integration.

Xcaliber retains its own dependency-free HTML/CSS/JavaScript frontend and Rust Tauri command
boundary. It does not adopt the reference architecture or packages.

## Product mapping

| Product concept studied | Original Xcaliber implementation |
|---|---|
| Guided first run | Hardware-first local onboarding with exact/adaptive/smaller-model choices |
| Projects | Local workspaces with isolated conversations and selected model profile |
| Model picker and loaded models | Reusable exact-K3 or compatible loopback profiles with explicit identity |
| Chat playground | Persistent local conversation, generation settings, exact one-shot handoff |
| Runtime lifecycle | Fixed allowlisted CLI and Docker operations with cancellation |
| API monitor | Bounded in-app request history, status, latency, model, endpoint, and token estimate |
| Run history | Bounded doctor/plan/pull/pack/run/Docker job history |
| Export | Password-free connection profile, diagnostics JSON, and conversation Markdown |
| Settings and privacy | Theme, density, reduced motion, local reset, and explicit data boundaries |
| Desktop preflight | CLI/Docker discovery, hardware planner, visible blockers, and runtime identity |

## Deliberate non-mappings

The following reference features are not represented as complete because Xcaliber does not
currently provide the required backend capability:

- fine-tuning and dataset recipe execution;
- image, audio, or video generation;
- hosted-provider credentials and remote inference;
- arbitrary tools, terminals, or shell execution;
- Hugging Face model browsing inside the desktop app;
- numerical or hardware certification for a full Kimi K3 checkpoint.

Adding empty screens for these features would misrepresent product readiness. They require
separate engines, security reviews, model licenses, tests, and release evidence.

## Security boundary

Model API profiles accept only `http://127.0.0.1` or `http://localhost`. Passwords are kept
in the current page session and are not persisted. The Rust backend maps typed requests to a
small command allowlist and never passes input through a command shell. Model files remain
outside the application bundle.
