# Superseded HTTP-oriented WebGeocalc review

Review date: 2026-11-11. **Historical archive: this review's HTTP/JSON acceptance criterion was withdrawn.** It is retained to preserve finding IDs, original numerical observations, and reference-service evidence. It is not the current defect list or release status.

The user clarified that the goal was identical computational functionality and results, without a local copy of NASA's HTTP service. Geocalc covers 13 geometry families and 11 finder families; TIME_CONVERSION is owned by the sibling Time project. F01 was therefore withdrawn. The corrected computational review is [WGC_CONFORMANCE_REVIEW.md](../WGC_CONFORMANCE_REVIEW.md). Maintained status is in [STRICT_PARITY_STATUS.md](../STRICT_PARITY_STATUS.md), and the reorganized source layout is described in [ARCHITECTURE.md](../ARCHITECTURE.md). Source line references in the earlier review describe its original snapshot.

## Original sources and executed checks

The review used the [NASA API documentation](https://wgc2.jpl.nasa.gov:8443/webgeocalc/documents/api-info.html), its [HTML snapshot](official-api-info.html), and [version response](official-version.json), which identified build 2.11.2 (5520 N67 19-OCT-2026). This is a historical observed build, not a claim about the present service.

The [original Cargo log](cargo-test.log) recorded 62 passing tests. The [field extraction](contract-fields.json) indexed the documented parameters. The [geometry sample summary](geometry-samples-summary.json) recorded 135 state cases, 2,184 representation comparisons, 42 separation cases, 18 phase cases, 54 size cases, six element cases, six solar-longitude cases, nine illumination cases, six intercept cases, and nine tangent cases. These are the original script's counts.

The frame/location sweep covered 87 frame configurations and 29 location/topocentric configurations. Both its [numerical summary](frames-locations-summary.json) and [raw requests/responses](frames-locations-live.json) are retained. Threshold-based successes were not exact equality certificates.

## Historical finding IDs

### F01 [P1] Absence of an HTTP/JSON service — withdrawn

The original review classified missing local routes, task state, kernel-set discovery, and wire-format responses as a P1 defect. That classification was outside the clarified scope. NASA HTTP remains useful for reference acquisition; a Rust API or CLI is a valid local computational interface. A local service, queue, cancellation endpoint, read-once result semantics, and REST status choices are not required by this project.

### F02 [P1] Sub-solar observer was discarded

The original function lacked an observer parameter; the CLI parsed the observer but used the Sun in the solar branch. For EARTH, observer MOON, ET=500000000, CN+S, Near point: ellipsoid:

| Quantity | Historical local result | NASA result |
|---|---:|---:|
| X / km | 5932.483663124207 | 5871.47926638 |
| Y / km | -1592.6384502252506 | -1804.42605126 |
| Z / km | -1711.7301093915944 | -1711.91506686 |
| Observer distance / km | 148327663.55082723 | 400413.90474339 |

The Y difference was approximately 211.788 km. Evidence: [targeted live results](targeted-live.json), [numerical defects](numeric-defects.json). Historical source locations and the required surface-light-time repair are retained in the corrected review's F02.

### F03 [P1] Corrected illumination used inconsistent surface epochs

For EARTH, observer MOON, longitude 10 degrees, latitude 20 degrees, altitude zero, ET=500000000, CN+S:

| Angle / degrees | Historical local result | NASA result |
|---|---:|---:|
| Incidence | 44.51338160284087 | 44.51005152 |
| Emission | 89.44143158456987 | 89.43628661 |
| Phase | 68.26765881952578 | 68.26778927 |

The worst difference was 0.005144975 degrees, above the old 1e-8 degree threshold. The old entry point also lacked a custom illuminator. Evidence: [targeted live results](targeted-live.json), [numerical defects](numeric-defects.json).

### F04 [P1] Finder longitude ignored loaded PCK metadata

Venus and [1e8,1e8,1e8] should have yielded east-positive longitude 45 degrees; the old finder path returned 315 degrees because it omitted the PCK rotation sense and explicit longitude overrides. Evidence: [coordinate reproduction](coordinate-repro.txt), [source](reproduce-coordinates.rs).

### F05 [P1] Empty local output could pass verification

With 135 NASA rows and zero local rows, unchecked zip truncation left maximum errors at zero. run_state returned (0.0,0.0,0.0,0) and the old pass expression was true. Missing finite-number checks also allowed max(0.0, NaN) to retain zero. Evidence: [false-pass reproduction](false-pass-repro.json).

### F06 [P1] Broad sub-point tolerances concealed errors

The original 42-point comparison measured 9.402e-3 km worst coordinate error and 1.415e1 km worst distance error. The sub-observer LT+S distance error was approximately 0.5902 km; the 14.15 km distance case used the Sun as sub-solar observer and was separate from F02. Allowed errors of 0.02 km and 20 km made the test pass. Evidence: [sub-point test log](subpoint-test.log).

### F07 [P1] Typed pointing branches were incomplete

The original API returned only a normalized target/observer position. VELOCITY, arbitrary VECTOR, PRESERVE_ORIGINAL, all coordinate representations, and multiple instrument FOV boundaries required a complete pointing entry point. The documented NASA example returned four boundary rows.

### F08 [P1] Finder conditions and result windows were incomplete

The reviewed code lacked RANGE/upperLimit, cyclic ranges, complement, expansion/contraction, duration filtering, and duration-unit processing. The earlier IS_IN_RANGE spelling was incorrect; the documented successful request used RANGE. A separate equality bug returned [-10,10] for f(t)=1e-9*t equal to zero instead of [0,0]. Evidence: [equality reproduction](equality-repro.txt).

### F09 [P1] Mission kernel and frame support was incomplete

The reviewed kernel implementation was limited to one DAF, SPK types 2/3, and J2000 segment frames. Multi-kernel ordering, DSK, CK, binary PCK, dynamic frames, and PCK-aware direction paths required further implementation. This describes the historical snapshot, not current capabilities.

### F09a [P1] Overlapping SPK segment priority was reversed

A native-written SPK had target 10001, center 0, J2000, coverage 0–10 seconds: first X=1 km, then X=2 km. At ET=5, CSPICE selected X=2 while the reviewed implementation selected X=1. Evidence: [SPK priority reproduction](spk-priority-repro.json).

### F10 [P2] Orbital elements omitted GM and frame modes

The original high-level calculation fixed center-only GM and J2000, omitting CENTER_ORBITER and arbitrary legal nonrotating reference frames. The comparator excluded the GM column. Manual GM addition or state rotation did not establish a verified complete calculation entry point.

### F11 [P2] Official API-availability claims were disproved

The earlier explanations that Pointing was inaccessible, sub-points always failed, and all 11 finder families were rejected were contradicted by successful documented payloads for all 25 families. Evidence: [live probes](live-probes.json). That successful reference availability alone did not establish local equality.

### F12 [P2] Missing kernels could be counted as passing tests

Some historical golden tests printed a missing-kernel message and returned successfully. Required kernels existed during the recorded 62-test run; the finding identified a false-green path in a fresh environment. Mandatory acceptance fixtures needed preflight failures.

## Historical HTTP transport observations

The following table records what NASA returned during that review. Its original local MISSING labels have been withdrawn as acceptance criteria; no fabricated local HTTP 404 response was used.

| Method | Relative path | Historical NASA observation |
|---|---|---|
| GET | / | Version/build JSON |
| GET | /kernel-sets | KernelSetDetails list |
| GET | /kernel-set/{id}/bodies | BodyData list for set 1 |
| GET | /kernel-set/{id}/frames | FrameData list for set 1 |
| GET | /kernel-set/{id}/instruments | Successful empty list for set 1 |
| GET | /kernel-set/{id}/surfaces | Successful empty list for set 1 |
| POST | /dsk-surfaces | Successful empty surfaces/sources/bodies |
| POST | /calculation/new | All 25 documented examples succeeded; empty payload produced a business error |
| GET | /calculation/{id} | COMPLETE, DISPATCHED, CANCELLED; unknown-ID business error |
| GET | /calculation/{id}/results | All 25 examples succeeded; repeated/cancelled result reads produced business errors |
| GET | /calculation/{id}/cancel | CANCELLED; repeated or dispatched cancellation produced business errors |

Evidence: [live probes](live-probes.json), [lifecycle probes](lifecycle-probes.json), [endpoint matrix](endpoint-matrix.csv). Observed business errors used HTTP 200 with JSON status=ERROR. LOADING_KERNELS, CALCULATING, and COMPLETE were observed; QUEUED, STARTING, FAILED, and EXPIRED were not artificially forced or claimed as tested. These are transport observations only.

## Historical calculation and field inventories

The [calculation matrix](calculation-matrix.csv) and [original documented requests](documented-requests.json) preserve all 25 families. The original matrix mixed computational gaps with HTTP/JSON omissions. Only the computational findings were relevant after the scope clarification. TIME_CONVERSION remained with the separate Time project.

The public-field inventory covered kernels and load order; time systems, formats and sampling; locations, topocentric frames and frame locus; finder conditions and result-window processing; direction/surface dictionaries; and output column IDs, types, units, row order and counts. Task status, expiry, cancellation and read-once service behavior were outside local computational scope. See [field extraction JSON](contract-fields.json) and [CSV](contract-fields.csv).

Two specification inconsistencies were preserved rather than guessed away: prose used intervalAdjustmentAmountUnits while the parameter table and successful examples used intervalAdjustmentUnits; the illumination-search parameter paragraph named GF_PHASE_ANGLE_SEARCH, while the successful example used GF_ILLUMINATION_ANGLES_SEARCH. Successful concrete requests were the evidence for resolving those discrepancies. Undocumented errorHandling/FAIL_ON_ERROR fields were not established public requirements. The documented GF illumination family allowed ELLIPSOID; DSK was not assigned as a gap for that family.

## Historical reproduction entry points

Run from the project root. These scripts preserve the original probes and failure reproductions; later repaired code may no longer reproduce an old failure. Numerical probes require the recorded kernels and spiceypy. Supply a configured proxy only when needed; a machine-specific proxy is not part of the API.

- cargo test --all-targets
- python tools/audit_wgc_contract.py --probe
- python tools/audit_wgc_targeted.py
- python docs/review-2026-11-11/reproduce-false-pass.py
- python docs/review-2026-11-11/reproduce-numeric.py
- python docs/review-2026-11-11/reproduce-spk-priority.py

The historical Rust coordinate and equality reproductions are reproduce-coordinates.rs and reproduce-equality.rs in this directory; their recorded text output remains available. Current verification instructions and open limitations belong to the maintained status and architecture documents, not this withdrawn HTTP-oriented plan.
