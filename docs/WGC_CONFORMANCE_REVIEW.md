# Historical WebGeocalc conformance review

Review date: 2026-11-11. **This is the pre-repair audit record, not the current defect list.** Finding IDs, original measurements, and evidence links are retained for traceability. Source locations refer to the reviewed snapshot.

Read [strict parity status](STRICT_PARITY_STATUS.md) for maintained implementation status and limitations, and [geometry parity evidence](review-2026-11-11/geometry-parity-status.md) for the later complete-family live audit. Historical tolerance-based passes below do not establish exact parity.

The source tree was later organized into domain folders; see [architecture](ARCHITECTURE.md). Original source paths below are retained as historical evidence.

## Scope and acceptance

Geocalc owns 13 Geometry Calculator families and 11 Geometry Finder families: 24 computational families, including kernel, frame, coordinate, correction, direction, and interval operations. The 25th documented family, TIME_CONVERSION, belongs to [STKCraft-time](https://github.com/spacelibres/STKCraft-time). Time version 0.1.15 was the original dependency snapshot, not the current version.

NASA HTTP is reference transport. A local REST/JSON server, job queue, cancellation service, or copy of NASA's wire protocol is outside the user's computational scope. **F01, which treated the absence of an HTTP server as a defect, was withdrawn.**

The acceptance rules established by this review remain applicable:

1. Pin identical kernel bytes, versions, loading order, coverage, bodies, frames, epochs, corrections, methods, and units.
2. Compare every returned quantity, including epochs, positions, velocities, angles, derivatives, light time, distance, GM, interval endpoints, ordering, and counts.
3. Require equality with the reference. Explicitly established NASA serialization must be checked separately from native CSPICE floating-point equality. Widened tolerances are diagnostics, not exact-parity passes.
4. Check no-intercept, empty-window, visibility, degeneracy, and unsupported-input behavior. Missing references, missing local results, NaN, missing rows, and missing columns must fail.
5. Track option coverage. One named function or passing example does not prove every branch of a family.

## Original evidence and its limits

The earlier [Cargo log](review-2026-11-11/cargo-test.log) recorded 62 passing tests. Some assertions allowed broad errors, so that count did not establish equality. The [geometry sample summary](review-2026-11-11/geometry-samples-summary.json) recorded 135 state cases, 2,184 state-representation comparisons, 42 separation cases, 18 phase cases, 54 size cases, six element cases, six solar-longitude cases, nine illumination cases, six intercept cases, and nine tangent cases. These are the original script's counts, not independent HTTP request counts.

The [frame/location summary](review-2026-11-11/frames-locations-summary.json) covered 87 frame and 29 location/topocentric configurations under old tolerances. The [live probes](review-2026-11-11/live-probes.json) obtained all 25 documented NASA examples; reference availability did not prove local equality. The original [Time log](review-2026-11-11/time-tests.log) recorded 108 passing tests, zero failures, and exit code zero; it did not prove exhaustive Time parity.

## Historical findings

### F02 [P1] Sub-solar calculation discarded the requested observer

Historical locations: src/surface.rs:174,185 and src/bin/wgc_geometry.rs:364. The function had no observer parameter and fixed the observer to the Sun. The CLI parsed an observer but discarded it. The observer determines light time, target epoch, and observer-to-point distance.

NASA reproduction: EARTH, observer MOON, ET=500000000, CN+S, Near point: ellipsoid.

| Quantity | Historical local result | NASA result |
|---|---:|---:|
| X / km | 5932.483663124207 | 5871.47926638 |
| Y / km | -1592.6384502252506 | -1804.42605126 |
| Z / km | -1711.7301093915944 | -1711.91506686 |
| Observer-to-point distance / km | 148327663.55082723 | 400413.90474339 |

The Y difference was approximately 211.788 km; the local distance was effectively a solar distance. Earlier tests used the Sun as observer. Evidence: [targeted results](review-2026-11-11/targeted-live.json), [numerical defects](review-2026-11-11/numeric-defects.json). The required repair was to propagate the real observer and iterate surface-point light time.

### F03 [P1] Corrected illumination used inconsistent epochs and vectors

Historical location: src/surface.rs:342, especially 353–360. Rotation at ET and separately corrected target/observer and target/Sun vectors did not reproduce SPICE surface-point semantics. The live comparator had exercised only NONE, and the entry point lacked a custom illuminator.

NASA reproduction: EARTH, observer MOON, longitude 10 degrees, latitude 20 degrees, altitude zero, ET=500000000, CN+S.

| Angle / degrees | Historical local result | NASA result |
|---|---:|---:|
| Incidence | 44.51338160284087 | 44.51005152 |
| Emission | 89.44143158456987 | 89.43628661 |
| Phase | 68.26765881952578 | 68.26778927 |

The worst difference was 0.005144975 degrees, exceeding the old 1e-8 degree threshold. Evidence: [targeted results](review-2026-11-11/targeted-live.json), [numerical defects](review-2026-11-11/numeric-defects.json).

### F04 [P1] Finder planetographic longitude lost PCK metadata

Historical locations: src/finder.rs:1176,1219,1248,1286 and src/coords.rs:92. The finder used planetographic_east_positive(body, None), ignoring retrograde rotation and BODY*_PGR_POSITIVE_LON overrides. For Venus and [1e8,1e8,1e8], the PCK required east-positive longitude 45 degrees; the old path returned 315 degrees. Evidence: [coordinate reproduction](review-2026-11-11/coordinate-repro.txt), [source](review-2026-11-11/reproduce-coordinates.rs). Every finder coordinate path needed the loaded PCK.

### F05 [P1] Missing local results could pass the comparator

Historical locations: tools/wgc2_geometry_compare.py:269,379,595,883,1217. Unchecked zip loops truncated comparisons, and maximum errors started at zero. Replacing crate_queries with an empty list left 135 remote rows and zero local rows, but run_state returned (0.0,0.0,0.0,0) and the old pass expression was true. max(0.0, NaN) could also retain zero. Evidence: [false-pass reproduction](review-2026-11-11/false-pass-repro.json). Required checks included exact counts, column IDs/order/types, nonzero coverage, and finite numbers.

### F06 [P1] Broad thresholds concealed corrected sub-point errors

Historical locations: src/surface.rs:110 and tests/subpoint_method_golden.rs:189. Target-center apparent states replaced surface-point iteration. A 42-point rerun measured worst coordinate error 9.402e-3 km and distance error 1.415e1 km. The sub-observer LT+S distance error was approximately 0.5902 km; the 14.15 km case used the Sun as sub-solar observer and is distinct from F02's lunar observer. Allowed errors of 0.02 km and 20 km made this a tolerance pass, not parity. Evidence: [sub-point test log](review-2026-11-11/subpoint-test.log).

### F07 [P1] Pointing exposed only a position unit vector

Historical locations: src/pointing.rs:20,35. The entry point immediately normalized target/observer position. It could not express VELOCITY, arbitrary VECTOR, PRESERVE_ORIGINAL, all coordinate outputs, or multiple FOV boundaries. The documented NASA example returned four boundary rows. A separate surface-ray resolver did not supply the missing typed pointing request.

### F08 [P1] Finder conditions and interval processing were incomplete

Historical locations: src/finder.rs:562,581,599,674. The condition model lacked documented RANGE/upperLimit and cyclic ranges. Scalar extremum adjustment did not implement result-window complement, expansion/contraction, duration filtering, or duration units. The earlier spelling IS_IN_RANGE was corrected to the request enum RANGE.

The equality path also used a time-convergence tolerance as a scalar band: f(t)=1e-9*t on [-10,10] returned the whole interval instead of [0,0]. Evidence: [equality reproduction](review-2026-11-11/equality-repro.txt).

### F09 [P1] Kernel and frame coverage did not support mission examples

Historical locations: src/spk.rs:219,293,655, src/frame.rs:271, src/direction.rs:133. The reviewed code used one DAF, SPK types 2/3, and J2000 segment frames. Multiple-kernel priority, DSK, CK, binary PCK, and dynamic frames were missing. Some pointing, ray, and finder paths still used static conversion without PCK context. Fixed FK or synthetic IK successes could not establish real mission coverage.

### F09a [P1] Overlapping SPK segments had reversed priority

Historical locations: src/spk.rs:568,580. Forward traversal locked in an earlier segment instead of SPICE's higher-priority later segment. A native-written SPK used target 10001, center 0, J2000, coverage 0–10 seconds: first X=1 km, then X=2 km. At ET=5, CSPICE returned X=2 and the reviewed code returned X=1. Evidence: [priority reproduction](review-2026-11-11/spk-priority-repro.json). This affected an already supported type, not just multi-kernel loading.

### F10 [P2] Orbital elements lacked GM and frame modes

Historical location: src/orbit.rs:52. The high-level entry point fixed GM to the center and the state frame to J2000. It lacked CENTER_ORBITER and arbitrary legal nonrotating frames. Manual GM addition or state rotation did not provide a verified complete entry point. The comparator also omitted the GM column.

### F11 [P2] API-availability explanations were disproved

Historical locations: docs/WGC_AUDIT.md:162,166,169 and README.md:733,741,746. Claims that Pointing was inaccessible, sub-points always failed, and all 11 finder types were unsupported were disproved by successful original documented payloads for all 25 families. Evidence: [live probes](review-2026-11-11/live-probes.json). The old inventory also mislabeled existing location/topocentric, frame-locus, orientation, and finder AZ/EL features as missing.

### F12 [P2] Missing fixtures could produce successful tests

Historical examples: tests/gf_golden.rs:45, tests/subpoint_method_golden.rs:48, tests/fov_golden.rs:40. Some tests printed a missing-kernel message and returned successfully. Kernels existed during the recorded 62-test run, so this finding did not invalidate that run; it identified a false-green path in a fresh environment. Acceptance resources needed mandatory preflight failures.

## Historical coverage and follow-up

The original family inventory recorded partial samples, known illumination/sub-point/finder errors, and incomplete pointing, GM, frame, mission-kernel, and interval branches. Tangent measurements included approximately 1.932 m in distance and 6.573e-5 degrees in angle. These are historical observations, not current support claims.

The repair sequence was to reproduce numerical defects, make verification reject omissions, implement computational options, add real mission kernels, and compare each branch with native CSPICE and explicitly formatted NASA outputs. Time remained separately tested. HTTP replication was excluded.

## Preserved evidence

- [Historical calculation matrix](review-2026-11-11/calculation-matrix.csv): a snapshot, not maintained support status.
- [Official HTML](review-2026-11-11/official-api-info.html) and [extracted fields](review-2026-11-11/contract-fields.json): specification evidence.
- [Superseded HTTP review](review-2026-11-11/superseded-http-review.md): withdrawn F01 and original transport inventory.
- [Strict parity status](STRICT_PARITY_STATUS.md): maintained status and limits.

The original review itself changed reports rather than production algorithms. Its statement that the directory was not a Git repository describes that snapshot only; repository and release status must be checked independently.
