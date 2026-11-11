# Contributing

Read [AGENTS.md](AGENTS.md), [the numerical contract](docs/NUMERICS.md) and [the architecture](docs/ARCHITECTURE.md) before changing calculations.

All maintained repository documentation, code comments, examples, changelog entries and commit messages use English. Keep changes grouped by responsibility under src/. Preserve existing public module paths through the root exports.

A numerical repair needs an independently generated failing reference before changing the algorithm. Record all outputs and semantic outcomes. Preserve kernel/source hashes and the reference environment. Do not widen thresholds, drop columns or ignore a failing test.

Before submission, run [the validation workflow](docs/VALIDATION.md). Update the API/MCP guides and Agent Skills when their contracts change. Every commit updates the version and a dated Keep a Changelog entry in the same commit. No change is left in an Unreleased section.

Describe the observable behavior, independent evidence, validation and remaining limits in a pull request. Never present a representative corpus as proof for every option. Use the [release procedure](docs/RELEASING.md) for versioned publication.
