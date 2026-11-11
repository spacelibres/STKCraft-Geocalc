# Strict WebGeocalc parity repair plan

User-authorized scope: repair all WebGeocalc computational features, using NASA CSPICE C source as an independent reference. Time remains owned by STKCraft-time. HTTP compatibility is out of scope.

## Architecture and acceptance

Keep the existing Rust implementation and typed interfaces. Pin CSPICE N0067, record source/kernel SHA-256 hashes, and generate reference data through the independent CSPICE binary used by spiceypy. Do not add hard-coded answer overrides, interpolation surrogates, excluded output columns, or widened error bounds. Exact WGC-visible output and internal CSPICE equality are separate assertions; a residual is a failure to achieve parity, even if an older test passes.

The initial audit directory had no Git metadata. On 2026-11-11 the user explicitly requested local commits and publication to the empty STKCraft-Geocalc GitHub repository; Git was initialized on main and existing work was preserved. STKCraft-time issues are recorded and handed off; the user explicitly forbids sibling repairs in this task.

## Ordered work

1. [x] Download and extract the official N0067 Windows C toolkit under target/reference; record URL, hashes, toolkit version and local reference runtime version in docs/reference/CSPICE_N0067.json. Keep reference C separate from production Rust.
2. [x] Add strict verifier regression tests in tools/tests: zero outputs, missing/extra rows or columns, non-finite numbers, mismatched identifiers and differing values must fail. Run the new tests before editing tools/wgc2_geometry_compare.py; remove false PASS paths and introduce an exact comparison mode whose failures cannot be overridden by legacy tolerances.
3. [x] Add generated overlapping SPK segment fixtures and Rust regression tests. Reproduce older/newer segments with different centers and overlapping/disjoint coverage, compare to CSPICE, and fix src/ephemeris/spk.rs selection by SPICE priority and center-chain semantics. Run cargo test --test spk_priority and the existing SPK/state tests.
4. [x] Reproduce the subsolar observer, subpoint light-time and illumination corrections against subslr/subpnt/illumf or ilumin C reference code. Add corrected-case golden vectors, including observer=MOON, NONE/LT/LT+S/CN/CN+S, non-Sun illuminators and both ellipsoid methods. Fix src/geometry/surface.rs and update src/bin/wgc_geometry.rs and callers as needed. Preserve public compatibility by explicit wrapper functions where practical. Do not hide residuals in thresholds.
5. [x] Add GF regression tests for the exact equality f(t)=1e-9*t, RANGE and angular wrap ranges, planetographic longitude direction from PCK, singleton/overlapping confinement windows, complement/expand/contract/filter operations. Fix src/search/finder.rs and its coordinate conversion paths; use CSPICE search results as independent reference. Keep time tolerance distinct from scalar values.
6. [ ] Complete direction and coordinate output branches (position, velocity, arbitrary vector, preserve-original magnitude, instrument FOV vectors), osculating GM modes and non-rotating reference frames, tangent correction locus and remaining supported shape/frame branches. Each new branch starts with a failing independent reference case and ends with recorded comparison results.
7. [ ] Work through all 24 geometry/GF types plus STKCraft-time ownership in docs/review-2026-11-11/calculation-matrix.csv. Add explicit option-level records and strict results. Missing functionality and unverified combinations remain incomplete; neither a function name nor a tolerance PASS closes an item.
8. [ ] Run cargo fmt --all -- --check, relevant Python verifier tests, cargo test --all-targets, and the strict NASA comparison. Review every changed file and update README/audit claims to describe only fresh evidence. Increment the smallest package version component after verified milestones; record the commit, tag, CI, registry and documentation publication outcomes separately.

## Execution boundaries

Surface geometry, GF and Python verification are independent edits and may run concurrently. SPK selection and shared public exports are coordinated by the root worker. Agents do not edit Cargo.toml, version metadata, README or shared exports without coordination. Integration and full regression checks are sequential. No whole-library completion claim is permitted while any option or reference comparison remains failing or unverified.
