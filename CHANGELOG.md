# Changelog

All notable changes are recorded in English using [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).
Versions 0.1.2 through 0.1.5 below describe verified local checkpoints before Git initialization; they were not separately published releases.

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
