# Project rules

## Scope and language

Implement NASA NAIF/SPICE WebGeocalc computational behavior and the requested GeoCalc web workspace. NASA HTTP protocol compatibility and complete numerical parity remain unverified. Time is owned by STKCraft-time; do not edit that repository when only a handoff is authorized.

All maintained content must be English: code comments, documentation, examples, Agent Skills, changelogs and commit messages. Original external captures are evidence, not editable numerical expectations.

For visual work use C:/Users/stone/.codex/skills/spacelibre-vi/SKILL.md and its approved source at https://github.com/spacelibres/vi. Backend/plain-text work does not require graphic changes.

## Organization and precision

Keep modules in the responsibility groups documented in docs/ARCHITECTURE.md. Preserve root public APIs. No unrounded numerical mismatch is accepted through a tolerance, output exclusion, input adjustment or approximation.

Add an independent native regression before a numerical repair. Match kernel bytes/load order and preserve all output fields, flags, ordering and errors. Keep position-only and full-state paths distinct.

## Every commit

- Increment the patch version by one for each verified update; synchronize Cargo.toml, Cargo.lock, CHANGELOG.md, CITATION.cff and release notes.
- Use Semantic Versioning. A breaking API change during 0.x requires a minor bump and an explicit migration note. An explicitly requested feature/minor release may also use a minor bump.
- Use dated headings such as ## [0.1.6] - 2026-11-11 with Added, Changed, Fixed, Removed, Deprecated or Security sections. Do not use Unreleased.
- Run the strict gate, all-feature/MCP tests, MSRV, rustdoc and package dry-run before publication. Missing data and skipped tests fail acceptance.
- Commit with Release X.Y.Z. Write a concise English body only when useful.
- Keep the working tree clean when reporting completion.

## Publishing

When publication is requested, push the verified commit to main, wait for CI, then create/push an annotated vX.Y.Z tag. Verify release binaries/checksums, crates.io and docs.rs separately. A missing registry credential is a blocker, not a successful skipped publication.

Never commit credentials. Do not rewrite published versions or tags. Keep complete parity false while docs/STRICT_REMAINING_GAPS.md contains open scope.
