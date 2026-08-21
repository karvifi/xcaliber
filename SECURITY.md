# Security policy

Xcaliber handles large local model files and can start local processes. Do not post
credentials, private model paths, or sensitive logs in public issues.

Report suspected vulnerabilities through GitHub's private vulnerability-reporting
feature for this repository when available. Include affected version, reproduction
steps, impact, and a minimal proof of concept. Please allow maintainers reasonable
time to investigate before public disclosure.

The Docker gateway binds to loopback by default and requires a local password.
Changing that binding can expose model access to a network and is the operator's
responsibility. Model weights are not distributed by this repository.
