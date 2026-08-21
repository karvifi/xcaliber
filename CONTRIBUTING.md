# Contributing to Xcaliber

Xcaliber is public so people can inspect it, adapt it to different hardware, and
improve local inference. Contributions are welcome.

## Before opening a change

1. Read `README.md`, `NOTICE`, and the architecture decisions in `docs/adr`.
2. Keep exact official-K3 execution separate from adaptive or surrogate modes.
3. Do not add hosted inference endpoints, secret collection, model weights, or
   unlicensed assets.
4. Preserve upstream notices when modifying bundled Apache-2.0 components.
5. Add or update tests for behavior changes.

## Local checks

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --release --locked
pwsh -File scripts/audit.ps1
node --check app/ui/app.js
```

The desktop crate is an independent workspace:

```powershell
cargo test --manifest-path app/src-tauri/Cargo.toml --locked
```

Real-checkpoint and hardware tests must state their required model revision,
hardware, driver, and expected evidence. Never convert an unexecuted gate into a
claimed pass.

## Pull requests

Describe the problem, the chosen execution mode, semantic impact, commands run,
and any untested hardware path. By contributing, you agree that your contribution
is licensed under AGPL-3.0-only unless it modifies an identified third-party
component that retains its compatible upstream license.
