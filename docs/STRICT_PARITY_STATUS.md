# Strict parity status

This project targets all NASA NAIF/SPICE WebGeocalc computational features and identical results. HTTP service compatibility is outside scope. Time conversion belongs to STKCraft-time. **Complete WebGeocalc conformance has not been certified.**

## Acceptance

The independent reference is CSPICE N0067. The complete official archive contains 2,229 C files; archive, selected source, native reference binary and kernel SHA-256 hashes are recorded in [the provenance manifest](reference/CSPICE_N0067.json). Production uses Rust, without linking CSPICE.

Unrounded native doubles, flags, classifications, counts, ordering and window endpoints must match exactly. Tests marked bit-exact compare IEEE-754 representations. Search convergence settings are inputs, not comparison tolerances. Missing kernels, skipped tests, absent columns and nonfinite values cannot establish conformance.

NASA output serialization is tested separately. Default raw transported-value failures remain recorded where NASA rounds its output. Explicit serialization equality does not prove internal binary64 equality.

## Verified implementation checkpoints

| Area | Independent reference coverage |
| --- | --- |
| SPK | All 16 N0067 SPKPVN types: 1/2/3/5/8/9/10/12/13/14/15/17/18/19/20/21. Newly added evaluators have 37,338 geometric states; type 10 also has 270 corrected state/position pairs. |
| SPK composition | Segment/file priority, common centers, 20-node buffer, 21 inertial frames and rotating source frames. SPKGPS uses position-only REFCHG; SPKGEO uses state FRMCHG. |
| CK | Types 1/2/3: 32,622 queries; type 4: 2,072; type 5: 19,072; type 6: 6,432. Additional byte-order and 832 SCLK frame cases pass. Independent sequential fuzz adds 30,000 exact queries. |
| PCK and frames | Binary PCK 2/3/20; text orientation; fixed, Euler, two-vector, product, Earth of-date and switch frames. See [dynamic frame scope](reference/DYNAMIC_FRAME_SCOPE.md). |
| Surface and shapes | Ellipsoid and unprioritized type-2 DSK direct geometry. See [DSK evidence](review-2026-11-11/dsk-exact.md) and [surface evidence](review-2026-11-11/surface-orbit-visibility-exact.md). |
| Dynamic geometry | 1,134 position-only SPKEZP cases, 360 direction cases, 48 ray cases and 864 tangent cases distinguish rotation-only and full-state paths. |
| GF | All 11 search families have native exact fixtures, including UDDC, RANGE scheduling, longitude boundaries and interval processing. |
| FOV | 1,865 GETFOV rows; 18,143 boundary predicates; 1,692 cross-frame/correction predicates; 144 window groups; 864 target-frame cases. Real Cassini attitude drives both live FOV search families. |

Counts describe actual fixture inputs, not every possible option combination. Fresh Debug/Release results and the source inventory are recorded by the strict gate; an older report does not certify changed sources.

## NASA family audit

The [live audit](review-2026-11-11/geometry-parity-status.md) checks all 24 non-time families: 13 geometry and 11 GF families. Geometry has 144 local query rows and 71 NASA calculations in 15 groups. GF has 12 requests, 18 nonempty intervals and 54 exact native start/stop/duration doubles. Explicit NASA serialization checks pass; raw transported-value differences are separately retained.

Actual KernelSet 1 load order is pinned through solar_system_v0072.tm. Full DE442 and selected required kernels are downloaded. This does not claim that every file in the 66-file set is locally present. Default Earth and Moon frames are verified from the actual kernels.

## Time and remaining scope

The dependency is pinned to published STKCraft-time 0.1.18. Earlier SCLK failures were handed off; the unchanged 42 SCE2C values, scalar parsing and 420 CK frame regressions now pass. No sibling Time source was changed by this project. [Historical evidence](review-2026-11-11/time-integration-gaps.md) remains available.

See [remaining gaps](STRICT_REMAINING_GAPS.md), [numerical contract](NUMERICS.md) and [validation instructions](VALIDATION.md). A versioned publication is a reproducible implementation checkpoint, not an assertion that the remaining gaps are closed.
