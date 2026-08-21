# Support

Xcaliber is a community project. There is no paid support or guaranteed response
time. Public support happens through GitHub Issues.

## Before opening an issue

Run these commands and save the output:

```powershell
xcaliber.exe --version
xcaliber.exe requirements
xcaliber.exe doctor --model-dir E:\Models\Kimi-K3 --json
xcaliber.exe plan --model-dir E:\Models\Kimi-K3 --json
docker version
docker compose version
```

Remove local paths, API keys, prompts, usernames, and other private information before
posting. Never upload model weights or proprietary checkpoint files.

Include the operating system, CPU, installed and available RAM, GPU and driver, model
drive type and free space, Docker version, exact command, full error, and whether the
problem occurs with the exact or adaptive runtime.

## Scope

Maintainers can help with reproducible Xcaliber source, planner, packaging, and runtime
defects. They cannot provide the Kimi K3 checkpoint, change Moonshot's model license,
guarantee a performance target on unmeasured hardware, or support hosted Kimi API
credentials. Questions about the official model and weights belong to Moonshot AI.

Security problems must be reported privately through
[GitHub Security Advisories](https://github.com/karvifi/xcaliber/security/advisories/new),
not a public issue.
