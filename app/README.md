# Xcaliber Desktop

Original Tauri 2 controller for the Xcaliber CLI and loopback Docker API. It contains no
Unsloth source, assets, branding, npm packages, or remote web content.

Keep the portable folder intact. The app looks for `xcaliber.exe`, `xcaliber-engine.exe`, and
`docker/compose.yaml` beside itself. It can run only fixed Xcaliber and Docker Compose actions;
there is no arbitrary shell bridge.

Standard Windows build prerequisites are Rust's MSVC target, Microsoft C++ Build Tools,
and WebView2. Build with:

```powershell
.\scripts\build-app.ps1
```

Model weights are never embedded. The app's local chat connects only to localhost under
its content security policy.

Xcaliber-specific desktop code is AGPL-3.0-only. Bundled runtimes retain the compatible
licenses and notices identified in the repository root.
