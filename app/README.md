# Xcaliber Desktop

Original Tauri 2 workspace for the Xcaliber CLI and loopback Docker API. It contains no
Unsloth source, assets, branding, npm packages, or remote web content. Unsloth Studio was
studied only as a clean-room product reference; the resulting feature map is documented in
[`docs/CLEAN-ROOM-STUDIO-MAP.md`](../docs/CLEAN-ROOM-STUDIO-MAP.md).

The Windows interface includes:

- local workspaces and reusable model profiles;
- persistent on-device conversations;
- exact K3 and compatible loopback model paths;
- hardware planning and K3 file preparation;
- fixed Docker service controls;
- request telemetry and local operation history;
- conversation, connection-profile, and diagnostic exports;
- appearance, privacy, and local-data controls.

The interface uses plain HTML, CSS, and JavaScript with Node's built-in test runner. It does
not include a frontend package manager or download web application code at runtime.

Keep the portable folder intact. The app lives at `Xcaliber.exe`; the CLI and exact engine
live under `runtime/`, and the Compose file lives under `docker/`. This separation is
required because Windows filenames are case-insensitive. The app can run only fixed
Xcaliber and Docker Compose actions; there is no arbitrary shell bridge.

Standard Windows build prerequisites are Rust's MSVC target, Microsoft C++ Build Tools,
and WebView2. Build with:

```powershell
.\scripts\build-app.ps1
```

Model weights are never embedded. The app's local chat connects only to localhost under
its content security policy.

Xcaliber-specific desktop code is AGPL-3.0-only. Bundled runtimes retain the compatible
licenses and notices identified in the repository root.
