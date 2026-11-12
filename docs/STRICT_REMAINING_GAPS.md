# Remaining conformance gaps

All 24 non-time WebGeocalc families have independent representative references. Complete option-level acceptance remains open. A function with the correct name, a rounded display match or a tolerance-based legacy test does not close a feature.

## Shared limits

- Runtime profile: the pinned native DLL itself reproduces the Windows Server 2022 system-UCRT drift. Version 0.1.8 selects static CRT math and retains all original expectations, with a 14,914-cell primitive guard and full main CI required before publication. Other operating-system, CPU and compiler profiles remain uncertified; see [runtime math evidence](reference/RUNTIME_MATH_INVESTIGATION.md). No tolerance or expectation replacement is accepted.
- Dynamic frames: wider TPARSE grammar/date error symbols, exhaustive type/cardinality errors and native cache-capacity limits remain incomplete. The pinned Time date resolver, general append, character types and a 120-case frame metadata matrix have native regressions; see [kernel-pool scope](reference/KERNEL_POOL_SCOPE.md) and [the frame scope](reference/DYNAMIC_FRAME_SCOPE.md).
- DSK: direct type-2 geometry, method-string LATSRF and J2000 SINCPT, selected surface-name/cache behavior, typed ray intersections and DSK-versus-point GFOCLT have independent native regressions. Complete method grammar and cache histories, full typed source/FOV products, illumination/subpoint facades and CLI/MCP model exposure remain incomplete. STATE_VECTOR point locations use a separate specification without a DSK model. See [surface-name evidence](review-2026-11-12/surface-names/README.md), [the explicit model scope](reference/DSK_MODEL_SCOPE.md) and [earlier DSK coverage](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.8/docs/review-2026-11-11/dsk-exact.md).
- CLI and MCP coverage is narrower than the Rust API where explicitly documented. Unsupported branches must return errors; no fallback ellipsoid or approximate orbit is accepted.
- Exact fixtures prove their exercised compiler/platform/kernel combinations. Linux/macOS build success alone does not prove equality to the pinned Windows native reference.
- Time is maintained independently. SPK10 now selects Time's explicit native historical JDUTC path; the recorded 33,696-group regression uses the original LSK. Altered LSK/frame/orbit products, full time representations and raw-pool errors remain open. Civil/scientific time policies stay distinct; see [SPK10 time scope](reference/SPK10_TIME_SCOPE.md).
- Native error-symbol, malformed-container and degenerate-input matrices are not exhaustive.

## Calculation-family acceptance map

| Family | Implemented reference path | Remaining acceptance work |
| --- | --- | --- |
| STATE_VECTOR | SPKEZR, eight coordinate representations, point locations | Full source/frame/location/shape cross-product |
| ANGULAR_SEPARATION | POINT/SPHERE and two directions | Additional source, shape and correction combinations |
| ANGULAR_SIZE | MAX_RADIUS, MIN_RADIUS, LIMB_EXTENTS | Full target/kernel/limb degeneracy matrix |
| FRAME_TRANSFORMATION | Fixed/PCK/CK/dynamic/switch; orientation and rates | Shared frame limits above and complete representation matrix |
| ILLUMINATION_ANGLES | Ellipsoid and direct DSK ILLUMF | DSK facade integration and complete coordinate/error matrix |
| PHASE_ANGLE | PHASEQ with independent illuminator | Full input/error/correction matrix |
| PLANETOCENTRIC_SOLAR_LONGITUDE | LSPCN | More body/kernel/epoch and error inputs |
| POINTING_DIRECTION | Position, velocity, vector, FOV boundaries | Full source/correction/representation product |
| SUB_SOLAR_POINT | Ellipsoid and direct DSK SUBSLR | Shape facade integration and full output matrix |
| SUB_OBSERVER_POINT | Ellipsoid and direct DSK SUBPNT | Shape facade integration and full output matrix |
| SURFACE_INTERCEPT_POINT | Explicit models and typed SINCPT results retaining misses | Full typed ray/FOV/model product and complete input-error matrix |
| TANGENT_POINT | Ellipsoid TANGPT; two correction loci | More near-singularity and mixed-source combinations |
| OSCULATING_ELEMENTS | OSCELT; CENTER/CENTER_ORBITER/CUSTOM GM | Full GM, state and orbital degeneracy matrix |
| GF_COORDINATE_SEARCH | GFPOSC | Complete coordinate, dynamic-frame and RANGE product |
| GF_ANGULAR_SEPARATION_SEARCH | GFSEP | Complete object/window/postprocessing product |
| GF_DISTANCE_SEARCH | GFDIST | Complete endpoint/window/options product |
| GF_SUB_POINT_SEARCH | GFSUBC ellipsoid | Full coordinate product; DSK is not silently substituted |
| GF_OCCULTATION_SEARCH | Explicit-frame GFOCLT POINT/ELLIPSOID/DSK-versus-point | Full frame/shape/error product and new CLI/MCP model exposure |
| GF_SURFACE_INTERCEPT_POINT_SEARCH | GFSNTC; typed dynamic directions | Complete direction/frame/condition products |
| GF_TARGET_IN_INSTRUMENT_FOV_SEARCH | GFFOV; real Cassini pointing | Additional mission/kernel/shape combinations |
| GF_RAY_IN_FOV_SEARCH | GFRFOV; real Cassini pointing | Additional dynamic ray/instrument combinations |
| GF_RANGE_RATE_SEARCH | GFRR | Complete relation and zero-range errors |
| GF_PHASE_ANGLE_SEARCH | GFPA | Complete illuminator/window/options matrix |
| GF_ILLUMINATION_ANGLES_SEARCH | GFILUM ellipsoid | Complete surface-point and target-frame matrix |
| TIME_CONVERSION | External STKCraft-time | Separate Time project acceptance |

## Closing a gap

Add an independent native or live-reference fixture, observe the failure when a repair is needed, preserve source algorithms and operation order, then compare every returned value and semantic outcome. Record kernel/source hashes and exact scope. Keep rejected and unverified cases visible. The full local gate, NASA serialization audit and feature-level completion claim are separate assertions.
