# Remaining conformance gaps

All 24 non-time WebGeocalc families have independent representative references. Complete option-level acceptance remains open. A function with the correct name, a rounded display match or a tolerance-based legacy test does not close a feature.

## Shared limits

- Dynamic frames: kernel date literals beginning with @, general kernel-pool append semantics beyond body aliases, quoted numeric-looking character values, exhaustive type/cardinality errors and native cache-capacity limits remain incomplete. See [the frame scope](reference/DYNAMIC_FRAME_SCOPE.md).
- DSK: direct type-2 unprioritized surface APIs are implemented. DSK integration into point-location endpoints, typed ray/coordinate facades, surface-name lookup and DSK-versus-point GF occultation remains incomplete. See [DSK coverage](review-2026-11-11/dsk-exact.md).
- CLI and MCP coverage is narrower than the Rust API where explicitly documented. Unsupported branches must return errors; no fallback ellipsoid or approximate orbit is accepted.
- Exact fixtures prove their exercised compiler/platform/kernel combinations. Linux/macOS build success alone does not prove equality to the pinned Windows native reference.
- Time is maintained independently. The NAIF TimeContext rejects UTC before its first LSK entry; SPK type-10 historical TLE epochs crossing that boundary require an explicit Time-owned contract decision.
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
| SURFACE_INTERCEPT_POINT | Ellipsoid and direct DSK SINCPT | DSK typed ray facade and complete input-error matrix |
| TANGENT_POINT | Ellipsoid TANGPT; two correction loci | More near-singularity and mixed-source combinations |
| OSCULATING_ELEMENTS | OSCELT; CENTER/CENTER_ORBITER/CUSTOM GM | Full GM, state and orbital degeneracy matrix |
| GF_COORDINATE_SEARCH | GFPOSC | Complete coordinate, dynamic-frame and RANGE product |
| GF_ANGULAR_SEPARATION_SEARCH | GFSEP | Complete object/window/postprocessing product |
| GF_DISTANCE_SEARCH | GFDIST | Complete endpoint/window/options product |
| GF_SUB_POINT_SEARCH | GFSUBC ellipsoid | Full coordinate product; DSK is not silently substituted |
| GF_OCCULTATION_SEARCH | GFOCLT POINT/ELLIPSOID | DSK-versus-point backend integration |
| GF_SURFACE_INTERCEPT_POINT_SEARCH | GFSNTC; typed dynamic directions | Complete direction/frame/condition products |
| GF_TARGET_IN_INSTRUMENT_FOV_SEARCH | GFFOV; real Cassini pointing | Additional mission/kernel/shape combinations |
| GF_RAY_IN_FOV_SEARCH | GFRFOV; real Cassini pointing | Additional dynamic ray/instrument combinations |
| GF_RANGE_RATE_SEARCH | GFRR | Complete relation and zero-range errors |
| GF_PHASE_ANGLE_SEARCH | GFPA | Complete illuminator/window/options matrix |
| GF_ILLUMINATION_ANGLES_SEARCH | GFILUM ellipsoid | Complete surface-point and target-frame matrix |
| TIME_CONVERSION | External STKCraft-time | Separate Time project acceptance |

## Closing a gap

Add an independent native or live-reference fixture, observe the failure when a repair is needed, preserve source algorithms and operation order, then compare every returned value and semantic outcome. Record kernel/source hashes and exact scope. Keep rejected and unverified cases visible. The full local gate, NASA serialization audit and feature-level completion claim are separate assertions.
