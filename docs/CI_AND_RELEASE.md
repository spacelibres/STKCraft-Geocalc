# CI and release verification

Reference bootstrap retains explicitly listed, original-byte kernel snapshots
when dated upstream files disappear. The original SHA256 remains mandatory;
missing or different snapshots fail. See
[snapshot provenance](reference/PINNED_KERNEL_SNAPSHOTS.md). A newer kernel is
not an acceptable replacement for a frozen numerical reference.

The Windows exactness job and portable build jobs answer different questions. Windows runs the complete Debug and Release test battery against committed native CSPICE N0067 fixtures. The portable jobs build every target, reject Clippy warnings, generate documentation and verify crate packaging. A separate Linux job checks the declared MSRV. A portable build is not evidence of bitwise numerical parity on that platform.

The native runtime discrepancy and static Windows reference profile are recorded
in [the runtime math investigation](reference/RUNTIME_MATH_INVESTIGATION.md).
The separate Runtime math diagnostic workflow records native/OS/FMA evidence
on Windows Server 2022 and 2025, and runs the independent primitive guard.
Its successful execution does not satisfy or bypass the strict acceptance job.

## Reproduce the Windows acceptance gate

Use Python 3.11 x86_64 and the stable Rust MSVC toolchain:

```powershell
python -m pip install --only-binary=:all: -r tools/requirements-reference.txt
cargo fetch --locked
python tools/bootstrap_reference.py
python tools/verify_strict.py
```

The bootstrap downloads the official Windows toolkit archive and the 38 pinned files required by the local gate: the main reference manifest, all 18 recorded Solar System FK files, and the recorded Cassini covering subset and metakernel. It does not download or claim coverage of every file in either NASA metakernel. Every download requires the committed SHA256. Unknown URLs, conflicting hashes, missing files and corrupted bytes fail the bootstrap. A mutable Earth-PCK alias is replaced by its recorded immutable equivalent.

Canonical provenance files under `docs/reference` and `docs/review-2026-11-11` are never rewritten by bootstrap. Machine-specific runtime paths go only into `target/bootstrap-report.json`. `--verify-only` checks an already provisioned checkout without downloading; `--kernels-only` supports data-only setup on other platforms.

The reference DLL must have the pinned SHA256 `7fd2dcc58d73ed5293847e67109897caba34240b8f9c38ad739c0b2ec6ce7715` and report `CSPICE_N0067`. The gate accepts `STKCRAFT_REFERENCE_DLL` as a runtime path override, but the hash check remains mandatory. If the historical manifest path is unavailable, the installed SpiceyPy native-library path is discovered automatically.

`verify_strict.py` rejects failed, skipped, ignored, missing-required-case or unexpectedly small Rust batteries. It executes all crate features, Python validator tests and formatting checks. It records source hashes before and after the run and fails if they change. Source code, test fixtures, C oracles, Windows build configuration, workflows, Agent Skills, all raw evidence and English documentation are included in this stability record.

Native diagnostics can interleave between a test name and its completion
status. The validator preserves the raw log, bounds each completion search by
the next test prefix, and requires an unambiguous success for every occurrence
of each required test name. Missing statuses, similarly suffixed names and a
later success after an earlier failure cannot establish acceptance.

The production dependency is the registry release `stkcraft-time =0.1.30`, not a sibling checkout. The gate resolves it through Cargo metadata, checks its downloaded crate archive against the Cargo.lock checksum, verifies the extracted Rust sources against that archive and records the version, registry source and checksum. Run `cargo fetch --locked` before the gate if Cargo's archive cache is absent.

Reports and logs are stored in `target/strict-verification`; GitHub Actions preserves them as the `windows-exact-report` artifact. Remaining parity limits are documented separately in [STRICT_REMAINING_GAPS.md](STRICT_REMAINING_GAPS.md). Passing CI does not change that scope.

The gate also hashes the three normal Release executables exercised through
`CARGO_BIN_EXE` by integration/MCP tests, checks their PE imports for dynamic
CRT dependencies and records `release_binary_sha256`. A successful Windows
job uploads those exact files as `windows-exact-binaries`. Missing binaries,
unexpected dynamic CRT imports or missing test output fail acceptance.

## Publish a release

The release workflow starts on a `vX.Y.Z` tag. The tag must match Cargo.toml,
English notes must exist at `docs/releases/vX.Y.Z.md`, and `.agents/skills`
must exist. It requires a successful same-repository main push run of `ci.yml`
for the exact tag commit. All five expected jobs must succeed; missing,
duplicated, skipped, cancelled or failed jobs are rejected.

`verify_release_ci.py` independently validates both source inventories against
the tag checkout and locked registry Time source, all gate command results,
raw Debug/Release/Python logs and the tested executable checksums. The release
then packages the exact Windows executables from that run. Linux/macOS
archives are built separately and do not inherit Windows bit-parity claims.
This evidence reuse does not accept another commit, branch or workflow and
does not omit any required scientific test.

Three platform archives include `wgc_geometry`, `wgc_gf`, `stkcraft-geocalc-mcp`, Agent Skills, licensing and English API/numerics/gap documentation. The verified `.crate` is also uploaded. The GitHub release publishes these four files plus `SHA256SUMS.txt`, using the versioned English release notes. Kernels and the native CSPICE oracle are test/data dependencies and are not bundled into the production binaries.

Configure the repository secret `CARGO_REGISTRY_TOKEN` with permission to publish `stkcraft-geocalc`. For an unpublished version, a missing secret fails the crates.io job explicitly; it is never a successful skip. An identical, already published version can be verified without a publish credential. Before publication, the newly packaged archive must match the verified release artifact. An already published version is accepted only if crates.io reports that identical package checksum. Unexpected HTTP responses or different existing bytes fail publication.

The workflows default to read-only repository access. Only the GitHub-release job receives `contents: write`; the crates.io token is provided only to the publication steps.
