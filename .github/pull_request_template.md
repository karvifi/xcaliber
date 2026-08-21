## Change

Describe the user-visible or engineering outcome.

## Semantics

- [ ] Preserves official Kimi K3 behavior, or is not model-execution code.
- [ ] Changes precision, routing, sampling, or execution semantics and is labeled adaptive.

## Validation

- [ ] cargo fmt --all -- --check
- [ ] cargo check --workspace --all-targets --locked
- [ ] cargo clippy --workspace --all-targets --locked -- -D warnings
- [ ] cargo test --workspace --release --locked
- [ ] scripts/audit.ps1
- [ ] Relevant desktop, Docker, checkpoint, or hardware tests are documented.

## Safety and licensing

- [ ] No model weights, private prompts, hosted Kimi credentials, or generated build directories are committed.
- [ ] New third-party code/assets include their license and provenance.
- [ ] Documentation does not claim an unexecuted certification or benchmark.
