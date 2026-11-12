# Strict parity status

This project targets all NASA NAIF/SPICE WebGeocalc computational features and identical results. HTTP service compatibility is outside scope. Time conversion belongs to STKCraft-time. **Complete WebGeocalc conformance has not been certified.**

## Acceptance

Version 0.1.8 selects static Windows CRT math following an independently
reproduced system-runtime discrepancy. The original Windows Server 2022 run
had thirteen failed tests in both Debug and Release; all expectations remain
unchanged. See [the runtime investigation](reference/RUNTIME_MATH_INVESTIGATION.md)
and [the reference profile](NUMERICS.md#windows-runtime-profile).

Publication requires successful main CI for the exact tag commit, including
the complete Debug/Release and MCP battery. The published Windows executables
must be byte-identical to those exercised by the Release tests. The additional
independent C primitive guard compares all 14,914 output cells, without a
tolerance. A passing local run or another platform's build is insufficient.

The independent reference is CSPICE N0067. The complete official archive contains 2,229 C files; archive, selected source, native reference binary and kernel SHA-256 hashes are recorded in [the provenance manifest](reference/CSPICE_N0067.json). Production uses Rust, without linking CSPICE.

Unrounded native doubles, flags, classifications, counts, ordering and window endpoints must match exactly. Tests marked bit-exact compare IEEE-754 representations. Search convergence settings are inputs, not comparison tolerances. Missing kernels, skipped tests, absent columns and nonfinite values cannot establish conformance.

NASA output serialization is tested separately. Default raw transported-value failures remain recorded where NASA rounds its output. Explicit serialization equality does not prove internal binary64 equality.

## Verified implementation checkpoints

The 0.1.9 text-kernel checkpoint adds ordered typed pools, native frame
validation timing and PCK partial-update regressions. Its
[scope record](reference/KERNEL_POOL_SCOPE.md) distinguishes checked outputs
from supporting captures. Kernel dates use the pinned Time resolver;
wider grammar and date error-symbol coverage remain incomplete.

The 0.1.10 model checkpoint adds LATSRF coordinates, all-field SINCPT results
through direct/typed paths and DSK-versus-point GFOCLT windows. See the
[model scope](reference/DSK_MODEL_SCOPE.md) for counts, raw evidence and open
branches. The full release gate must pass for its final source inventory;
representative native coverage does not establish complete parity.

| Area | Independent reference coverage |
| --- | --- |
| SPK | All 16 N0067 SPKPVN types: 1/2/3/5/8/9/10/12/13/14/15/17/18/19/20/21. Newly added evaluators have 37,338 geometric states; type 10 also has 270 corrected state/position pairs. |
| SPK composition | Segment/file priority, common centers, 20-node buffer, 21 inertial frames and rotating source frames. SPKGPS uses position-only REFCHG; SPKGEO uses state FRMCHG. |
| CK | Types 1/2/3: 32,622 queries; type 4: 2,072; type 5: 19,072; type 6: 6,432. Additional byte-order and 832 SCLK frame cases pass. Independent sequential fuzz adds 30,000 exact queries. |
| PCK and frames | Binary PCK 2/3/20; text orientation; fixed, Euler, two-vector, product, Earth of-date and switch frames. See [dynamic frame scope](reference/DYNAMIC_FRAME_SCOPE.md). |
| Surface and shapes | Ellipsoid and unprioritized type-2 DSK direct geometry; 2,352 LATSRF calls, 1,296 all-field SINCPT calls through two paths, and 864 DSK/point GFOCLT queries. See [model scope](reference/DSK_MODEL_SCOPE.md) and [earlier DSK evidence](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.8/docs/review-2026-11-11/dsk-exact.md). |
| Dynamic geometry | 1,134 position-only SPKEZP cases, 360 direction cases, 48 ray cases and 864 tangent cases distinguish rotation-only and full-state paths. |
| GF | All 11 search families have native exact fixtures, including UDDC, RANGE scheduling, longitude boundaries and interval processing. |
| FOV | 1,865 GETFOV rows; 18,143 boundary predicates; 1,692 cross-frame/correction predicates; 144 window groups; 864 target-frame cases. Real Cassini attitude drives both live FOV search families. |

Counts describe actual fixture inputs, not every possible option combination. Fresh Debug/Release results and the source inventory are recorded by the strict gate; an older report does not certify changed sources.

The 0.1.11 SPK10 checkpoint adds 33,696 native historical query groups
and 370,656 exact state/position/light-time cells through Time's explicit
JDUTC path. See [the exact time scope](reference/SPK10_TIME_SCOPE.md).

The 0.1.14 surface-name checkpoint records 11,338 native method-parser cases,
1,065 LATSRF method/frame cases, 24 absent-DSK cases and 756 SINCPT method,
frame and correction cases. Additional captures cover surface lookup, cache
history, fixed-width names, numerical fallback, invalid radii and selected
inertial frames. [Evidence and scope](review-2026-11-12/surface-names/README.md)
identify the native library, kernels and capture hashes. These cases do not
certify every input or expose DSK methods in CLI/MCP.

## NASA family audit

The [live audit](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.8/docs/review-2026-11-11/geometry-parity-status.md) checks all 24 non-time families: 13 geometry and 11 GF families. Geometry has 144 local query rows and 71 NASA calculations in 15 groups. GF has 12 requests, 18 nonempty intervals and 54 exact native start/stop/duration doubles. Explicit NASA serialization checks pass; raw transported-value differences are separately retained.

Actual KernelSet 1 load order is pinned through solar_system_v0072.tm. Full DE442 and selected required kernels are downloaded. This does not claim that every file in the 66-file set is locally present. Default Earth and Moon frames are verified from the actual kernels.

## Time and remaining scope

The compatible dependency range starts at STKCraft-time 0.2.0; the release lockfile records the exact verified version. Earlier SCLK failures were handed off; the unchanged 42 SCE2C values, scalar parsing and 420 CK frame regressions remain active. Under explicit authorization, Time kernel-date arithmetic and grammar repairs were independently verified and released as 0.1.27/0.1.28; see [Time integration](reference/TIME_KERNEL_DATES.md). [Historical evidence](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.8/docs/review-2026-11-11/time-integration-gaps.md) remains available.

See [remaining gaps](STRICT_REMAINING_GAPS.md), [numerical contract](NUMERICS.md) and [validation instructions](VALIDATION.md). A versioned publication is a reproducible implementation checkpoint, not an assertion that the remaining gaps are closed.
