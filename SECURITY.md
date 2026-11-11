# Security policy

## Reporting

Report vulnerabilities privately through https://github.com/spacelibres/STKCraft-Geocalc/security/advisories/new when private reporting is available, or contact a repository maintainer privately. Do not disclose sensitive reproduction data in a public issue.

Include affected versions, a minimal input, expected/actual behavior and the impact. No response-time or supported-version guarantee is implied before maintainers establish one.

## Scope

Kernel readers parse untrusted DAF/DAS and text data. MCP reads caller-selected local files and launches only the bundled calculation executables; it does not authorize arbitrary shell commands. Production forbids unsafe Rust. Numerical correctness defects without a security impact should use the ordinary issue tracker with independent reference evidence.

Credentials, local private kernels and access tokens must never enter committed evidence or release archives.
