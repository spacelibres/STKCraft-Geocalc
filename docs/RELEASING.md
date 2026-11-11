# Versioning and release procedure

GitHub releases are based on Git tags; GitHub does not impose a universal version-number scheme. This project uses [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), following the STKCraft-time repository conventions.

## Versioned update

1. Review the numerical/API changes and remaining gaps.
2. Increment the patch component for the next verified update. A breaking 0.x API change requires a minor bump; reset the patch component for minor/major releases.
3. Synchronize Cargo.toml, Cargo.lock, CITATION.cff, README examples and docs/releases/vX.Y.Z.md.
4. Add a dated CHANGELOG.md entry with applicable standard categories. Never overwrite a published version.
5. Run the full [validation](VALIDATION.md), including MCP, MSRV, documentation and cargo publish --locked --dry-run.
6. Commit as Release X.Y.Z and push to main. Require the CI run to pass.
7. Create an annotated tag vX.Y.Z pointing to that exact commit and push it.
8. Confirm the GitHub Release, per-platform archives and SHA256SUMS.txt.
9. Confirm the same version on crates.io and the docs.rs build. Keep publication failures visible.

## Distribution contents

Git contains independent fixtures, generators, provenance and raw NASA evidence. The crates.io archive contains production source, public guides, licenses, source provenance and Agent Skills; large fixtures, downloaded kernels and the CSPICE archive are excluded. Reproduce the reference tests from a Git checkout.

Release archives contain the geometry/GF executables and optional MCP executable, README, LICENSE, ATTRIBUTION, CHANGELOG and Agent Skills. Platform build artifacts are not additional numerical certification.

## Credentials

GitHub Actions uses its scoped repository token for release assets. crates.io requires CARGO_REGISTRY_TOKEN in the repository secrets or an authenticated local Cargo credential provider. Do not print, commit or embed the credential. Publication must fail explicitly if required credentials are absent.

The official [Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html) describes mandatory package metadata, the package-size limit and the package verification performed by cargo publish.
