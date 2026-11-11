# Historical WebGeocalc coverage audit

**Archived pre-repair snapshot.** This document preserves the original numerical measurements and numbered repair notes 1–15. Its statements about missing options, remaining work, dependency versions, and source locations describe the reviewed snapshot, not the current implementation. Broad numerical tolerances used by that audit did not establish exact parity.

Use [strict parity status](STRICT_PARITY_STATUS.md) for maintained support and limitations, [the later conformance review](WGC_CONFORMANCE_REVIEW.md) for historical findings F02–F12, and [complete-family live evidence](review-2026-11-11/geometry-parity-status.md) for subsequent validation. The source tree was later organized into domain folders; see [architecture](ARCHITECTURE.md). Original source paths below are historical.

The earlier claims that Pointing, sub-points, and all finder types could not be called through NASA's public API were disproved by [successful documented requests](review-2026-11-11/live-probes.json). They are retained below only as withdrawn explanations. HTTP/JSON service replication is outside the computational scope. The old IS_IN_RANGE spelling was corrected to RANGE; unsupported, undocumented errorHandling fields were not acceptance requirements. The old Tangent Point DSK entry was not evidence that WGC offered that combination.

Date: 2026-11-11
WebGeocalc build: `2.11.2 (5520 N67 19-OCT-2026)`

## Sources used

| Source | Location | Use |
|---|---|---|
| WGC Web API documentation | `https://wgc2.jpl.nasa.gov:8443/webgeocalc/documents/api-info.html` | Authoritative list of calculation types, parameters, enumerations, and output columns |
| WGC API root | `https://wgc2.jpl.nasa.gov:8443/webgeocalc/api` | Version, kernel sets, live calculations |
| WGC GWT client bundles | `target/wgc/*.cache.js` | UI labels and enum strings used by the web client |
| CSPICE N0067 source | `target/cspice.zip` | Reference implementation of every routine named by WGC |
| WGC live service | `https://wgc2.jpl.nasa.gov:8443/webgeocalc/api` | Numerical comparison for the calculation types the public API accepts |
| `STKCraft-time` | `../STKCraft-time` | Time-system and time-format implementation |

The WGC API documentation describes **25 calculation types**:

- 13 Geometry Calculator types
- 11 Geometry Finder types
- 1 Time Conversion type

## Historical executive conclusion

The original audit identified named implementation entry points across the 25 documented calculation types, but that inventory was insufficient to prove complete option coverage or exact results. TIME_CONVERSION belonged to the separate STKCraft-time project. The statements below summarize the original inventory only.

The original, subsequently superseded summary was:

> All WebGeocalc calculation types are implemented for ephemeris objects in
> SPICE frames, with triaxial-ellipsoid or point shapes, and are numerically
> verified against CSPICE. The WGC API's location specifications, topocentric
> frames, alternative state/orientation representations, interval adjustment
> and filtering, kernel-set management, time sampling, and JSON response
> formatting are only partially covered.

The earlier claim that "all WebGeocalc functionality" was copied was too
broad and is withdrawn.

## Historical calculator matrix

Legend:

- **Implemented**: the geometry is present in this crate.
- **Missing options**: WGC input fields or output columns not covered.
- **Verified**: numerical comparison actually performed.

### Geometry Calculator

| WGC type | Implemented | Missing WGC options | Verified |
|---|---|---|---|
| `STATE_VECTOR` | `state_corrected`, `apparent_state_in`, `state_columns`/`state_columns_with_pck` | `targetType`/`observerType` locations (`FIXED_POINT`, `CONSTANT_VELOCITY_POINT`), `TOPOCENTRIC_AT_POINT`, `frameLocus` | Live WGC: rectangular J2000, 135 cases, position `2.4e-7 km`; all 8 representations, 4 frames, `NONE` and `LT+S`, 2184 columns, worst `4.99e-9` against its display tolerance |
| `ANGULAR_SEPARATION` | `angular_separation`, `angular_separation_shaped` | `TWO_DIRECTIONS` | Live WGC, `POINT` and `SPHERE`: 42 cases, `4.8e-9 deg`. CSPICE `trgsep`: 14 shaped cases `1.6e-14 rad` |
| `ANGULAR_SIZE` | `angular_size_with_method` | Custom `targetFrame` other than the IAU frame supplied by the PCK | Live WGC, all three methods, 54 cases, `1.43e-8 deg` |
| `FRAME_TRANSFORMATION` | `Frames::pxform`, `Frames::sxform` | `EULER_ANGLES`, `ANGLE_AND_AXIS`, `SPICE_QUATERNION`, `OTHER_QUATERNION`, `MATRIX_FLAGGED`, `MATRIX_ALL_ONE_ROW`; angular velocity representations; `timeLocation`; aberration-corrected frame epochs | Live WGC, matrix only, 2 cases, `4.1e-9` |
| `ILLUMINATION_ANGLES` | `illumination_angles` | Surface-point specification by lat/lon/colatitude/radius/altitude, `DSK`, non-Sun `illuminator`, output observer distance, time-at-point, light time, LTST | Live WGC, body-fixed Cartesian input, 9 cases, `4.6e-9 deg` |
| `PHASE_ANGLE` | `phase_angle` | None for the documented reception-only mode | Live WGC, 18 cases, `4.8e-9 deg` |
| `PLANETOCENTRIC_SOLAR_LONGITUDE` | `planetocentric_solar_longitude` | None for the documented reception-only mode | Live WGC, 6 cases, `4.6e-9 deg` |
| `POINTING_DIRECTION` | `pointing_direction`, `pointing_ra_dec` | Direction specifications (`POSITION`, `VELOCITY`, `VECTOR`, frame axis, instrument boresight/FOV), `vectorMagnitude`, coordinate representations including `AZ_EL` | CSPICE `spkezp`, 90 directions, `7.6e-14` |
| `SUB_SOLAR_POINT` | `sub_solar_point`, `sub_solar_point_with_method` | `DSK`, output state representations, incidence/emission/phase, time-at-point; corrected-epoch residual (see defect 9) | CSPICE `subslr`, both ellipsoid definitions |
| `SUB_OBSERVER_POINT` | `sub_observer_point`, `sub_observer_point_with_method` | `DSK`, output state representations, incidence/emission/phase, time-at-point; corrected-epoch residual (see defect 9) | CSPICE `subpnt`, both ellipsoid definitions |
| `SURFACE_INTERCEPT_POINT` | `surface_intercept_point`, `surface_intercept_ray`, `surface_intercept_points` | `DSK`; output state representations and derived angles | Live WGC, direction-to-object, 6 cases, point `3.4e-8 km`. All six direction forms vs CSPICE `sincpt` |
| `TANGENT_POINT` | `tangent_point`, `tangent_points` | `SURFACE_POINT` correction locus; `DSK`; output state representations and derived angles | Live WGC, direction-to-object, 9 cases, angle `6.6e-5 deg`. All six direction forms vs CSPICE `tangpt` |
| `OSCULATING_ELEMENTS` | `osculating_elements`, `elements_from_state` | `gmType = CENTER_ORBITER`, arbitrary `referenceFrame` (always J2000) | Live WGC, center GM, 6 cases, angles `4.9e-9 deg` |

### Geometry Finder

| WGC type | Implemented | Missing WGC options | Verified |
|---|---|---|---|
| `GF_COORDINATE_SEARCH` | `find_position_coordinate(_with_radii)` | `AZ/EL` coordinates, location specifications, interval adjustment/filtering | CSPICE `gfposc` |
| `GF_ANGULAR_SEPARATION_SEARCH` | `search_separation`, `search_separation_shaped`, `find`-style scalar search | Interval adjustment/filtering | CSPICE `gfsep` with `POINT` and `SPHERE` |
| `GF_DISTANCE_SEARCH` | `find_distance` | Location specifications, interval adjustment/filtering | CSPICE `gfdist` |
| `GF_SUB_POINT_SEARCH` | `find_sub_point_coordinate` | `INTERCEPT_ELLIPSOID`, `DSK`, interval adjustment/filtering | CSPICE `gfsubc` |
| `GF_OCCULTATION_SEARCH` | `find_occultation` | `DSK`, interval adjustment/filtering | CSPICE `gfoclt` |
| `GF_SURFACE_INTERCEPT_POINT_SEARCH` | `find_surface_intercept_ray_coordinate` | Instrument/FOV/frame direction types, `DSK`, interval adjustment/filtering | CSPICE `gfsntc` |
| `GF_TARGET_IN_INSTRUMENT_FOV_SEARCH` | `find_target_in_fov`, `find_target_in_fov_shaped` | CK/time-dependent frames, interval adjustment/filtering | CSPICE `gftfov`; `fovtrg` with `POINT` (80 epochs) and `ELLIPSOID` (672 cases, all flags agree) |
| `GF_RAY_IN_FOV_SEARCH` | `find_ray_in_fov` | Instrument/FOV/frame direction types, CK/time-dependent frames, interval adjustment/filtering | CSPICE `gfrfov` |
| `GF_RANGE_RATE_SEARCH` | `find_range_rate` | Location specifications, interval adjustment/filtering | CSPICE `gfrr` |
| `GF_PHASE_ANGLE_SEARCH` | `find_phase_angle` | Interval adjustment/filtering | CSPICE `gfpa` |
| `GF_ILLUMINATION_ANGLES_SEARCH` | `find_illumination_angle` | `DSK`, interval adjustment/filtering | CSPICE `gfilum` |

### Time Conversion

| WGC type | Implemented | Missing WGC options | Verified |
|---|---|---|---|
| `TIME_CONVERSION` | `stkcraft-time` | WGC custom format strings, `WEEK_TOW`, `CALENDAR_DOY`, GPS0 seconds output, SCLK-as-output-format combinations | `stkcraft-time` WGC comparison binary and tests |

## Historical enum coverage

| WGC enum | Values | Status |
|---|---|---|
| `aberrationCorrection` | `NONE`, `LT`, `LT+S`, `CN`, `CN+S`, `XLT`, `XLT+S`, `XCN`, `XCN+S` | All supported. `PHASE_ANGLE` and `PLANETOCENTRIC_SOLAR_LONGITUDE` reject transmission, matching WGC. |
| `stateRepresentation` | `RECTANGULAR`, `RA_DEC`, `LATITUDINAL`, `PLANETODETIC`, `PLANETOGRAPHIC`, `CYLINDRICAL`, `SPHERICAL`, `AZ_EL` | All eight supported, including both `AZ_EL` flags, with the WGC column order and analytic coordinate rates. Verified live: 2184 columns. |
| `orientationRepresentation` | `EULER_ANGLES`, `ANGLE_AND_AXIS`, `SPICE_QUATERNION`, `OTHER_QUATERNION`, `MATRIX_ROW_BY_ROW`, `MATRIX_FLAGGED`, `MATRIX_ALL_ONE_ROW` | All seven. Column layouts measured live: three rows for the row-by-row forms, one row of nine for `MATRIX_ALL_ONE_ROW`, and Euler angles in `angle3, angle2, angle1` order. |
| `angularVelocityRepresentation` | `NOT_INCLUDED`, `VECTOR_IN_FRAME1`, `VECTOR_IN_FRAME2`, `EULER_ANGLE_DERIVATIVES`, `MATRIX` | All five. The vector forms append `AV X/Y/Z` and a magnitude, repeated on every row of the multi-row matrix forms. `MATRIX` emits the 6x6 state transformation for the three matrix orientations and is silently dropped for the angle and quaternion forms, exactly as the live API behaves. `EULER_ANGLE_DERIVATIVES` requires `EULER_ANGLES` with the same axis sequence, which the live API also enforces. |
| `locationTypes` | `OBJECT`, `FIXED_POINT`, `CONSTANT_VELOCITY_POINT` | All three. Point locations accept all eight `coordinateRepresentation` forms, the constant-velocity epoch, and linear extrapolation. Verified live: 29 queries. |
| `frameTypes` | `SPICE_FRAME`, `TOPOCENTRIC_AT_POINT` | Both. The topocentric frame matches CSPICE `azlcpo`: `+Z` is the outward normal at the observer's nearest point on the ellipsoid and `+X` points at the body's north pole. |
| `locusTypes` | `CENTER`, `OBSERVER`, `TARGET` | All three, matching the three branches of CSPICE `spkcvo`. They only differ when the output frame rotates. |
| `shapeTypes` | `ELLIPSOID`, `POINT`, `SPHERE`, `TWO_TARGETS`, `TWO_DIRECTIONS` | `ELLIPSOID`, `POINT` and `SPHERE`; `TWO_DIRECTIONS` missing. |
| `methodTypes` | `NEAR_POINT_ELLIPSOID`, `INTERCEPT_ELLIPSOID`, `NEAR_POINT_ELLIPS1`, `INTERCEPT_ELLIPS1`, `INTERCEPT_DSK` | Both ellipsoid methods for sub-points and FOV targets; DSK and the `ELLIPS1` variants are not implemented. |
| `occultationTypes` | `FULL`, `ANNULAR`, `PARTIAL` | All supported, plus `ANY`. |
| `relationalConditions` | `GREATER_THAN`, `LESS_THAN`, `IS_IN_RANGE`, `IS_GLOBAL_MAXIMUM`, `IS_GLOBAL_MINIMUM`, `IS_LOCAL_MAXIMUM`, `IS_LOCAL_MINIMUM` | `>`, `<`, equality/±adjustment, `ABSMAX`, `ABSMIN`, `LOCMAX`, `LOCMIN`; explicit `IS_IN_RANGE` spelling missing. |
| `resultIntervalAdjustment` | `NO_ADJUSTMENT`, `EXPAND_INTERVALS`, `CONTRACT_INTERVALS` | Not implemented. |
| `resultIntervalFiltering` | `NO_FILTERING`, `FILTER_INTERVALS` | Not implemented. |
| `gmTypes` | `CENTER`, `CENTER_ORBITER`, `CUSTOM` | `CENTER` and `CUSTOM`; `CENTER_ORBITER` missing. |
| `directionVectorTypes` | `INSTRUMENT_BORESIGHT`, `INSTRUMENT_FOV_BOUNDARY_VECTORS`, `REFERENCE_FRAME_AXIS`, `VECTOR_IN_INSTRUMENT_FOV`, `VECTOR_IN_REFERENCE_FRAME`, `DIRECTION_TO_OBJECT` | All six via `RayDirection`; frames are resolved through `Frames`, so `IAU_<body>` names are rejected. |
| `vectorMagnitude` | `UNIT`, `PRESERVE_ORIGINAL` | Unit only. |
| `timeSystems` | `UTC`, `TDB`, `TDT`, `TT`, `SPACECRAFT_CLOCK`, `GPS` | `stkcraft-time` covers these scales; WGC JSON wrapper not implemented. |
| `timeFormats` | `CALENDAR`, `CALENDAR_DOY`, `JULIAN`, `SECONDS_PAST_GPS0`, `SECONDS_PAST_J2000`, `SPACECRAFT_CLOCK_TICKS`, `SPACECRAFT_CLOCK_STRING`, `WEEK_TOW`, `CUSTOM` | Partially covered by `stkcraft-time`; `CALENDAR_DOY`, `WEEK_TOW`, GPS0 seconds, and custom formats are not WGC-shaped. |

## Historical API inventory, outside the service-replication scope

| Feature | Status |
|---|---|
| Kernel-set selection (`kernel-set/{id}`, priority ordering, meta-kernels) | Not implemented; callers supply file paths |
| Time-system input/output conversion in the geometry API | Delegated to `stkcraft-time`; no WGC request/response plumbing |
| `times` / `intervals` / `timeStep` / `timeStepUnits` sampling | Not implemented; callers pass explicit `et` values or confinement windows |
| `errorHandling` (`FAIL_ON_ERROR`, `OMIT_ERRORS`, `IGNORE_BUT_REPORT_ERRORS`) | Not implemented; Rust `Result` only |
| Output `columns` and `outputID` schema | Not implemented; typed Rust structs instead |
| `outputTimeSystem`, `outputTimeFormat`, `outputSclkId`, `outputTimeCustomFormat` | Not implemented |
| DSK surfaces (`dskSurfaces`, `DSK` shape) | Not implemented |
| CK attitude segments, SCLK in geometry, dynamic frames | Not implemented |
| Topocentric frames and fixed/constant-velocity locations | Implemented: `PointLocation`, `state_between`, `express`, and `topocentric_rotation` |
| Interval expansion/contraction and filtering | Not implemented |

## Historical tolerance-based live measurements

The reported OVERALL: PASS below is the original comparator's output. It is preserved as evidence of that run, not accepted as strict parity. Later findings included zero-row false passes and omitted columns; see [F05](WGC_CONFORMANCE_REVIEW.md#f05-p1-missing-local-results-could-pass-the-comparator). The subset file de442s.bsp in this historical command must not be confused with the actual full DE442 kernel pinned by subsequent audits.

Run:

```bash
python tools/wgc2_geometry_compare.py \
    --spk target/kernels/de442s.bsp --insecure
```

Result on 2026-11-11: `OVERALL: PASS`.

Kernel set 1 is **not** `pck00011.tpc` alone: the `/kernel-sets`
documentation says it loads the generic text PCK **and**
`mars_iau2000_v1.tpc` after it, which overrides the Mars pole and prime
meridian. Comparisons that use `IAU_MARS` therefore load both files; with
`pck00011.tpc` alone the Mars rotation differs by `2.9e-5` and it is the
kernel stack, not the crate, that differs. `tools/fetch_kernels.py`
downloads `mars_iau2000_v1.tpc` for this reason.

| Calculation | Items | Worst difference |
|---|---|---|
| State Vector | 135 | position `2.4e-7 km`, velocity `5.0e-9 km/s`, light time `4.9e-9 s` |
| State Vector representations | 2184 | worst `4.99e-9` against the per-column display tolerance (0.499 of it); coordinates, speeds and rates at WGC's eight-decimal floor |
| Angular Separation | 42 | `4.8e-9 deg` over `POINT`/`SPHERE` combinations |
| Phase Angle | 18 | `4.8e-9 deg` |
| Angular Size, all three methods | 54 | `1.43e-8 deg` |
| Osculating Elements | 6 | angles `4.9e-9 deg`, distance `1.8e-7 km` |
| Planetocentric Solar Longitude | 6 | `4.6e-9 deg` |
| Illumination Angles | 9 | `4.6e-9 deg` |
| Surface Intercept Point | 6 | point `3.4e-8 km`, distance `9.8e-9 km` |
| Tangent Point | 9 | angle `6.6e-5 deg`, distance `1.9e-3 km` |
| Frame Transformation (seven orientations × five angular velocities) | 87 | worst cell `4.558e-9`, which is the eight-decimal display floor |
| Point locations and topocentric frame | 29 | worst cell `1.7e-8` over object/point and constant-velocity targets and observers, all three loci, and both `AZ_EL` flag combinations |

Historical API-availability explanations, subsequently withdrawn:

- **Pointing Direction**: the live endpoint requires a nested `DIRECTION`
  `SpecType` group that could not be exercised through the public API.
  Verified against CSPICE `spkezp`.
- **Sub-observer point**: the live endpoint returns an internal
  `NullPointerException` for every payload tried. Verified against CSPICE
  `subpnt`.
- **Sub-solar point**: same API failure. Verified against CSPICE `subslr`.
- **All 11 Geometry Finder types**: the live API rejects the finder
  calculation types with “calculation type is not supported”. Verified against
  the CSPICE routines WebGeocalc calls.

## Historical numbered repair notes

Notes 1–15 retain their original measurements and chronology. References to a defect that "remains open" mean open at that point in the review. They do not override the maintained status document or later exact native tests.

1. **Angular Size used the equatorial radius only.** WGC defaults to
   `MAX_RADIUS` and supports `MIN_RADIUS` and `LIMB_EXTENTS`.
   `angular_size_with_method` now implements all three; `angular_size` uses
   the WGC default. Live agreement is `1.43e-8 deg` over 54 cases.
2. **Phase Angle accepted transmission corrections.** WGC documents
   reception-only corrections. The function now rejects `X*`.
3. **Planetocentric Solar Longitude accepted transmission corrections.**
   WGC documents reception-only corrections. The function now rejects `X*`.
4. **Occultation of bodies on opposite sides of the observer** produced a
   degenerate image plane. The classifier now returns “no occultation” for
   bodies in opposite hemispheres.
5. **Planetographic longitude direction was hardcoded** to “east for the
   Earth, Moon and Sun, west otherwise”. CSPICE `recpgr` derives the
   direction from the body's spin sense, so Venus (retrograde) and Uranus
   (retrograde) were wrong. `coords::planetographic_east_positive` and
   `Pck::planetographic_east_positive` now follow `recpgr`: an explicit
   `BODY<id>_PGR_POSITIVE_LON` wins, then the Earth/Moon/Sun exception,
   then the sign of the first-degree prime-meridian coefficient.
6. **PCK nutation phases assumed `(angle, rate)` pairs stored under the
   body's own id.** `pck00011.tpc` stores Mars' phases under
   `BODY4_NUT_PREC_ANGLES` and makes them quadratic with
   `BODY4_MAX_PHASE_DEGREE = 2`. Both rules are now implemented, which
   moved the Mars rotation matrix from `2.4e-2` to `7.1e-12` against
   CSPICE `sxform`.
7. **`spkezr` frame semantics were missing.** A non-inertial output frame
   is evaluated at the observer's epoch for the frame centre, and the
   body-fixed velocity carries `(1 + sigma*dlt/dt)` on the frame's rotation
   term. Without both, `IAU_EARTH` light-time states were off by `38.7 km`
   and `2.6e-6 km/s`. Both are now in `SpkKernel::apparent_state_in`.
8. **The frame epoch used the wrong light time.** The first fix used the
   observer-target light time; CSPICE uses observer-to-**frame-centre**.
   The two coincide only when the target is the frame centre, which every
   earlier test happened to satisfy. Probing `spkezp` with the target and
   the frame centre deliberately different (`SUN` as seen from `EARTH` in
   `IAU_MOON`) showed a `215 km` error, which is now removed: the same
   probe agrees to `4e-5 km`.
9. **Corrected sub-points still differ from `subpnt`/`subslr`.** CSPICE
   iterates the target epoch using the light time from the observer to the
   *sub-point*, and for the sub-solar point it takes the Sun's apparent
   position as seen from the sub-point (`spkcpo`). This crate evaluates the
   frame at the observer-to-target epoch. `NONE` matches to `3e-8 km`;
   corrected cases leave a sub-point residual of `9.4e-3 km` and an
   altitude residual of `1.4e1 km` (the altitude is a range to a body
   `1.5e8 km` away). Recorded by
   `tests/subpoint_method_golden.rs`, which prints the per-class numbers.
10. **A legal two-term polynomial override was discarded.** `first3`
    required three coefficients, so `BODY499_PM = ( 176.630 350.89198226 )`
    from `mars_iau2000_v1.tpc` was ignored and the Mars prime meridian
    stayed on the `pck00011.tpc` model, while the pole switched to the IAU
    2000 values: a hybrid matching neither kernel. The live
    `FRAME_TRANSFORMATION` sweep exposed it as `9.2e-3` in the rotation.
    `first3` now pads missing higher-order terms with zero, as SPICE does,
    which brings the same sweep to `4.5e-9`.
11. **The angular velocity columns had the wrong shape.** The previous
    `describe` returned a single 36-column row for `MATRIX` under every
    orientation, and put the angular velocity on the first row only. The
    live API emits six rows of six for `MATRIX_ROW_BY_ROW` and
    `MATRIX_FLAGGED`, one row of 36 for `MATRIX_ALL_ONE_ROW`, no matrix at
    all for the angle and quaternion forms, and repeats the angular
    velocity on every row of the multi-row matrix forms. All four rules
    are now implemented and pinned by `tests/orientation_columns.rs`.
12. **Frame rotations had no analytic derivative.** `Frames::pxform`
    ignored its epoch, so the frame graph could not express a body-fixed
    frame at all. `Frames::pxform_with_derivative` now composes the
    rotation and its derivative through the frame chain with the product
    rule, taking the body rate from the PCK IAU model and accepting
    `IAU_<id>` frame names. `tests/frame_derivative_golden.rs` compares it
    with CSPICE `sxform` over fourteen frame pairs: the static pairs agree
    to `1e-16`, the Moon to `2e-13` radians, and the fast rotators to
    `5e-12`, which is the sine-and-cosine floor of a prime-meridian angle
    of `2e4` radians rather than a model difference.
13. **The topocentric zenith used the wrong normal.** The frame was built
    from the ellipsoid gradient at the observer's own position. CSPICE
    `azlcpo` takes the normal at the observer's **nearest point** on the
    ellipsoid, and for an oblate body those differ: the azimuth and
    elevation were off by `3.2e-6` and `7.8e-6` degrees. Using the nearest
    point removes the difference, leaving `1.0e-12` radians.
14. **A state-columns radius lookup was demanded when it was not
    needed.** `state_columns_with_pck` required a PCK entry for the body
    even for `RECTANGULAR`, so a frame centred on a body with no PCK
    entry could not be used. The lookup is now lazy and the error is still
    raised for the ellipsoidal representations.
15. **The output-frame rotation was inverted in the new `express`.**
    The first version called `pxform_with_derivative(frame, J2000)`, which
    maps the frame into J2000 instead of the state into the frame. The
    inertial test cases passed because `J2000` is its own inverse and the
    error only appeared with a rotating output frame, where it was a
    `6.2e5 km` residual. Caught by the golden test, not by inspection.

## Historical follow-up list

The following priority list was recorded during the original audit; it is not a current work queue:

1. ~~`AZ_EL` coordinates and state representations.~~ Done: all eight
   representations, WGC column order, analytic rates, and CSPICE
   `spkezr` frame semantics.
2. ~~`SPHERE` shapes for angular separation and FOV targets.~~ Done:
   `SPHERE` for `trgsep`/`gfsep`, and `ELLIPSOID` FOV targets for all four
   FOV shapes, verified flag-for-flag against `fovtrg` over 672 cases.
3. ~~`INTERCEPT_ELLIPSOID` sub-point method.~~ Done: both definitions for
   sub-observer and sub-solar points. The corrected-epoch iteration that
   CSPICE performs around the sub-point is defect 9 and remains open.
4. ~~Instrument/FOV/frame direction types for surface intercept and
   tangent point.~~ Done: all six `directionVectorType` forms, with
   `antiVectorFlag` and `vectorAbCorr`.
5. ~~Frame orientation representations and angular velocity.~~ Done: all
   seven orientations and all five angular velocity representations, with
   the live column layouts, `Frames::pxform_with_derivative`, and the
   measured frame sweep recorded above.
6. ~~`FIXED_POINT` / `CONSTANT_VELOCITY_POINT` locations and topocentric
   frames.~~ Done: `src/location.rs` with all eight coordinate systems,
   `state_between`/`express`, all three `frameLocus` values, and the
   topocentric frame; verified against `spkcpo`, `spkcvo`, `spkcpt`,
   `spkcvt`, `azlcpo`, and 29 live queries.
7. Geometry Finder interval adjustment/filtering.
8. CK frames and DSK shapes.
9. Time Conversion WGC formats (`WEEK_TOW`, `CALENDAR_DOY`, GPS0 seconds,
   custom formats).

## Historical reproduction

The raw API documentation was converted to text with:

```bash
python tools/wgc_api_docs.py
python tools/wgc_audit_extract.py
python tools/wgc_output_ids.py
```

The extracted sections are under `target/wgc/sections/` and are not
committed.
