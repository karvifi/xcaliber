# Changelog

All notable public changes are recorded here. Xcaliber uses semantic versions for its
own control-plane, CLI, desktop, and packaging code. Model weights and their upstream
versions are separate.

## 1.3.0 - 2026-08-21

### Added

- Expanded the original Windows application into a local model studio with workspaces,
  reusable model profiles, persistent conversations, generation controls, API monitoring,
  operation history, safe exports, appearance settings, and local-data controls.
- Added a clean-room product map documenting which desktop-studio concepts apply to
  Xcaliber and which capabilities are deliberately not represented as complete.
- Added dependency-free frontend tests for local endpoint enforcement, stored-state bounds,
  metrics, and conversation export.

### Fixed

- Separated the portable desktop executable from the CLI runtime directory so Windows
  case-insensitive paths cannot overwrite the app or make it recursively launch itself.
- Made the loopback HTTP test server consume the complete request before closing, removing
  a Windows connection-reset race without weakening the production client assertions.

## 1.2.0 - 2026-08-21

### Added

- Original Xcaliber logo, wordmark, repository banner, application icons, and brand
  artwork.
- First-run desktop guide and a practical smaller-local-model connection path.
- Loopback-only CLI chat for OpenAI-compatible local endpoints.
- Release-only Docker Compose file for the public GHCR image.
- NSIS installer configuration that embeds the CLI, exact engine, packing tool,
  Compose file, licenses, and validation record.
- Automated Windows release, GHCR publication, checksums, and build provenance.
- Dependabot, issue forms, pull-request checklist, ownership, support, conduct, and
  product-readiness documentation.
- Deeper release audit coverage for versions, assets, installers, workflows, and
  release Compose safety.

### Changed

- Portable Windows CLI packages now include the release Compose file and brand assets.
- The desktop controller can locate CLI and Compose resources from an installed Tauri
  bundle.
- The desktop service start action uses the published image path without forcing a
  source rebuild.

### Validation boundary

This release does not claim a full official-checkpoint run, Docker execution on the
validation host, GPU numerical parity, hardware performance certification, code
signing, or multi-computer K3 execution.

## 1.1.0 - 2026-08-21

- First public Xcaliber source release with the Rust CLI and exact engine, adaptive
  Docker runtime, Tauri controller, hardware planner, weightless tests, and explicit
  full-model release gates.
