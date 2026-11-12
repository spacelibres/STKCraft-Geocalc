# Changelog

All notable changes are recorded in English using [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).
Versions 0.1.2 through 0.1.5 below describe verified local checkpoints before Git initialization; they were not separately published releases.

## [0.1.11] - 2026-11-12

### Added

- Independent native historical SPK10 regression: 52 generated segments,
  33,696 query groups and 370,656 unrounded state/position/light-time cells,
  including all nine corrections, two observers and four output frames.
- Original kernel/source/toolchain captures and mandatory exact regression
  names; preserve existing SPK10 expectations unchanged.

### Fixed

- Initialize SPK10 from Time's explicit native scalar JDUTC, removing the
  historical civil-coverage blocker and integer Epoch/calendar detour.
  Retain distinct full-state and position-only paths and public signatures.

### Changed

- Pin the actual published Time 0.1.30 registry package and document the
  IERS-only/TableDriven native model requirement. Other time policies
  produce explicit errors; no alternate model or kernel is substituted.
- Record the historical scope and remaining altered-LSK/frame/error
  products. Complete WebGeocalc parity remains false.

## [0.1.10] - 2026-11-12

### Added

- Explicit-model LATSRF coordinate construction with independent native points and error outcomes.
- Typed shape-aware SINCPT results retaining every ray's missing-intercept flag and ordering.
- Explicit-frame ellipsoid/DSK-versus-point GFOCLT searches, including foreground point transits; original APIs remain available.
- Independent native captures and mandatory strict regressions for 2,352 coordinate calls, 1,296 intersections through two entry points, 864 DSK occultation queries and 162 existing ellipsoid/point windows.

### Fixed

- Report SINCPT light time from the returned observer-to-surface vector's native norm divided by CLIGHT, preserving the point and target epoch.
- Preserve the DSKDATANOTFOUND symbol for absent selected surface data.
- Restore reproducible reference bootstrap after NAIF removed a dated Earth PCK URL. Stage the original, hash-identical NASA WGC snapshot; never substitute the newer kernel.

### Changed

- Pin published Time 0.1.29 after checking its registry archive; its Rust sources are byte-identical to the native-tested 0.1.28 implementation.
- Distinguish WebGeocalc surface-point specifications from state-vector point locations. Document numeric surface selection, narrower CLI/MCP coverage and remaining conformance gaps without claiming complete parity.

## [0.1.9] - 2026-11-12

### Added

- Independent N0067 typed kernel-pool, append, partial-load and frame metadata/error captures; exact fixed/dynamic position and state transformations.
- Native PCK repeated-load captures and regressions preserving earlier successful assignments and invalidating removed or character-replaced constants.
- Kernel-date integration with exact registry Time 0.1.28, original native values/rejection outcomes, and direct numeric reads that avoid decimal formatting/reparsing.

### Fixed

- Preserve case-sensitive pool names, character types, numeric-looking quoted strings, general ordered append and native parsing errors.
- Defer fixed-frame validation to transformation queries, preserve native identity shortcuts, and check required metadata with native error ordering.
- Rebuild text-PCK caches from the effective pool, including after failed loads; retain binary PCK records.
- Exclude generated Python bytecode from source inventories while retaining original binary and diagnostic captures unchanged.

### Changed

- Document corrected validation timing and partial-load semantics; retain existing public Rust signatures.
- Pin the verified Time registry release 0.1.28 and validate future exact pins against Cargo metadata and archive bytes. Keep complete WebGeocalc parity false, including wider date grammar/error symbols, full option/error products and DSK facade integration.

## [0.1.8] - 2026-11-11

### Added

- Independent C primitive regression: 13,909 rows and 14,914 raw binary64 outputs, including signed zero, subnormals and domain boundaries; preserve every original computational expectation.
- Native Windows Server 2022/2025 runtime captures proving that the pinned CSPICE DLL itself drifts with the system UCRT, plus actual linker library provenance.
- Release evidence validation covering main commit identity, all CI jobs, raw logs, complete source hashes and the exact tested Windows executables.

### Fixed

- Select static UCRT math for Windows MSVC reference builds; document explicit Cargo registry/downstream configuration rather than relying on an ignored dependency configuration.
- Include all public English guides and reference scope documents in the crate; link large original captures to immutable versioned GitHub evidence.
- Parse interleaved native test diagnostics without losing required completion statuses; reject similarly suffixed names, missing statuses and failures hidden by later successes.

### Changed

- Publish the Windows executables exercised by the complete Release gate, verified by SHA256, instead of rebuilding them on another runner.
- Require successful main push CI for the exact tag commit and revalidate its complete evidence before release packaging.
- Keep complete WebGeocalc parity false; platform, option and error-matrix limits remain explicit.

## [0.1.7] - 2026-11-11

### Added

- Isolated native/Rust dynamic/static UCRT and FMA diagnostics with unchanged VSEP and PROP2B expectations, native DLL verification, linker/library provenance and raw-bit evidence.
- Windows Server 2022/2025 diagnostic artifacts, separate from numerical acceptance.
- A documented publication blocker for thirteen remote exactness failures in both Debug and Release; no numerical repair or publication is claimed.

### Fixed

- Preserve original review evidence, nested captures and the remote CI failure report byte-for-byte in Git.

## [0.1.6] - 2026-11-11

### Added

- Type-2 DSK containers, unprioritized multi-segment intercepts and direct SINCPT, SUBPNT, SUBSLR and ILLUMF calculations with independent native references.
- Dynamic two-vector, product, Earth of-date and switch frames, including explicit SPK and clock context.
- Optional stdio MCP server, typed geometry/GF tools, protocol regressions and project-specific Agent Skills.
- English API, architecture, numerical contract, validation, release, contribution, support, security and attribution documentation.
- Pinned-reference bootstrap, GitHub CI/release workflows, versioned release notes and a local version/changelog/strict-gate commit check.

### Changed

- Organize src into ephemeris, kernels, frames, shapes, geometry and search responsibilities while retaining existing public module paths.
- Pin the published STKCraft-time 0.1.18 dependency; remove the machine-local sibling path from the package.
- Separate the distributable source package from large reference fixtures, downloaded kernels and the official CSPICE archive.
- Record immutable Earth orientation data and complete source/runtime/kernel provenance.

### Fixed

- Preserve rotation-only REFCHG versus full-state FRMCHG arithmetic for dynamic SPK positions, directions, rays and tangent/surface geometry.
- Supply selected SPK context to ephemeris-defined frames in both calculation bridges.
- Use the full observer frame context for stellar corrections when the observer has a rotating or dynamic SPK source.
- Match the native SPK17 eccentricity-boundary acceptance and rejection sequence.

This release is an implementation checkpoint. Full option-level WebGeocalc conformance remains open in docs/STRICT_REMAINING_GAPS.md; raw NASA rounding differences remain visible separately from output-format equality.

## [0.1.5] - 2026-11-11

Complete CK types4/5/6 alongside the existing types1/2/3.

- Compare2,072 CK4,19,072 CK5 and6,432 CK6 native queries exactly,
  including gap selection, interpolation windows, quaternion derivatives
  and native reader reuse behavior.
- Add native references for both byte orders and832 SCLK-driven frame
  transforms. All11 focused CK tests pass in Debug and Release.
- Preserve the32,622-query CK1/2/3 baseline and existing clock regressions.

DSK integration, dynamic frame options and the final complete gate remain
open. This directory has no Git metadata; no commit could be created.

## [0.1.4] - 2026-11-11

Complete the16 SPK evaluators dispatched by native N0067 SPKPVN.

- Add native difference-line types1/21, two-body type5, generic Chebyshev
  type14, J2 precessing type15 and interval packet type19.
- Integrate type10 SGP4, near/deep-space resonances, TEME to J2000 and
  explicit STKCraft-time LSK context. Keep time conversion in the time crate.
- The newly added evaluators have37,338 independent geometric states;
  type10 also has270 corrected state/position pairs. Focused Debug and
  Release regressions pass with exact comparisons.
- Preserve source input/record priority when an unknown newer segment
  masks an older segment, using an explicitly unknown type999 fixture.

This is a verified kernel milestone; DSK and further frame/option work
remain open. The directory has no Git metadata and no commit was created.

## [0.1.3] - 2026-11-11

Verified native SPK evaluator and named-frame milestone. The full conformance
requirement remains open, including independently reproduced time integration failures.

- Add type17 equinoctial propagation, type18 packet interpolation and type20
  velocity-integral records: 3,024, 6,720 and 2,205 exact native states respectively.
- Preserve requested-frame segment accumulation and rotating source frames in
  context-aware geometric, corrected state and position-only paths; 2,592 paired
  SPKEZR/SPKPOS cases pass with all nine corrections.
- Enforce native FOV visibility width limits even with empty confinement windows.
- Verify these focused regressions in Debug and Release with exact comparisons.

The supplied directory remains outside Git; no commit can be created here.

## [0.1.2] - 2026-11-11

Strict CSPICE parity repairs and additional WebGeocalc calculation branches.
This version is a verified implementation milestone, not complete WebGeocalc certification.

- Preserve SPICE interpolation, segment priority, multi-file load order,
  mixed inertial frames and 20-node center-chain arithmetic.
- Separate SPKAPO position corrections from SPKACS state corrections;
  preserve frame-specific correction and derivative accumulation order.
- Replace custom surface, tangent, occultation, FOV and GF approximations
  with source algorithms and independent exact reference fixtures.
- Add complete pointing sources, dual-direction separation, GM modes,
  coordinate Jacobians, point observers, correction loci and frame units.
- Add binary PCK type-2 records, file/segment priority, both byte orders,
  frame class-ID lookup and default body-frame associations.
- Add exact GF RANGE scheduling and window postprocessing. Keep generic
  value-only sampling helpers separate from native conformance paths.
- Reject incomplete comparisons and preserve all NASA raw differences;
  provide an explicitly selected output-format compatibility check.

Current coverage and remaining backend/option gaps are tracked in
`docs/STRICT_PARITY_STATUS.md` and `docs/STRICT_REMAINING_GAPS.md`.

The supplied directory has no Git metadata; no repository or commit was created.
