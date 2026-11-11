# Exact surface, orbit, and visibility evidence — 2026-11-11

This records same-kernel comparisons against CSPICE N0067 through SpiceyPy 8.3.0. Every passing numerical comparison below requires equal floating-point bits or equal output values; no acceptance tolerance is used. Boolean/classification results require exact equality. This evidence covers the listed implementations and fixtures. It does not establish complete parity with all WebGeocalc calculations, configurations, or transport formats.

## Source implementations and checked coverage

| Implementation | Independent reference cases | Checked outputs | Result |
|---|---:|---|---|
| SUBPNT / SUBSLR / ILLUMG | 342 | 2,394 point, surface-epoch, surface-vector, and angle components | Exact |
| NEARPT | 308 | 1,232 point and signed-altitude components; 205 exterior and 103 interior/origin/central-plane cases | Exact |
| SINCPT | 108 | 756 point, epoch, and surface-vector components | Exact |
| SINCPT grazing NONE recovery | 1 | Initial rotation and position plus final point, epoch, and surface vector | Exact |
| GEOREC | 6 | 18 coordinates | Exact |
| LSPCN | 60 | Solar longitude, four bodies, three epochs, five reception corrections | Exact |
| TANGPT | 432 | 5,184 point, altitude, range, surface point, epoch, and surface-vector components | Exact |
| Internal NPEDLN | 200 | 800 nearest-point and distance components on arbitrary triaxial ellipsoids | Exact |
| SAELGV | 100 | 600 ellipse semi-axis components | Exact |
| OSCELT / OSCLTX synthetic states | 163 | 1,956 elements/range/speed/GM components | Exact |
| Osculating observations | 3,402 | 40,824 elements/range/speed/GM components; Moon/Earth, Earth/Sun, Mercury/Sun; all 21 built-in inertial frames, all nine aberration corrections, three GM modes | Exact |
| ZZOCCED | 1,000 | Designated-front occultation classifications for general rotated triaxial ellipsoids | Exact |
| EDLIMB | 100 | 900 ellipse components | Exact |
| ZZASRYEL | 200 | 800 minimum/maximum angular-separation and extremizing-point components | Exact |
| ZZELVUPY | 1,000 | Ellipse/pyramid visibility predicates | Exact |
| INELPL | 1,000 | Intersection presence; all six point components for 682 non-empty intersections | Exact |
| FOVTRG, ellipsoidal targets | 672 | Circular, elliptical, rectangular, polygonal FOV predicates; 480 visible cases | Exact |
| Original ray-direction fixtures | 84 | 252 raw resolved-vector components, 156 found-point components, 52 hits and 32 misses | Exact |
| Focused occultation states/windows | 783 states / 162 windows | Classification and complete returned window endpoints | Exact |

The original surface fixtures also now require exact equality: 15 near-side points, 42 method-specific subpoints, six center-ray intercepts, six illumination cases, and 48 tangent cases. Their old numerical bounds have been removed. The source-reference generators for surface ranges now call CSPICE VNORM rather than a hand-written `sqrt(sum(component²))` expression.

The tangent corpus covers both TANGENT POINT and SURFACE POINT correction loci, all nine corrections, Earth and Moon, three epochs, and rays that intersect, miss, or point away from the body. The orbital corpus includes circular, equatorial, retrograde, elliptic, parabolic, near-parabolic, and hyperbolic states. CENTER and CENTER_ORBITER GM values come from the loaded constants kernel; CUSTOM uses the explicitly supplied parameter.

## Material corrections

- Surface epochs are iterated using observer-to-surface distance. SUBSLR uses the actual observer, and illuminator states are evaluated from the surface location with its rotational velocity. ILLUMG/ILLUMF supports an arbitrary illuminator and returns surface epoch/vector and visibility/illumination flags.
- Source operation ordering is retained for NEARPT, SURFPT, VNORM, VSEP, STELAB/VROTV, NPEDLN and its ellipse/projection helpers, and OSCELT/OSCLTX. Hyperbolic mean anomaly retains its sign; non-computable/parabolic extended axis/period values follow OSCLTX zero conventions.
- SINCPT retains the final FOUND flag after near-limb tangent recovery. Its recovery also runs with NONE correction when the initial relative-chain geometry misses: the reference recomputes the observer through absolute barycentric states, whose cancellation can change a grazing result. These behaviors were necessary to match all focused occultation windows.
- Ellipsoid occultation uses ZZOCCED bounding-cone and transformed-limb logic. Its ZZASRYEL search follows the official 320/400 initial evaluations and golden-section termination; those are the reference algorithm, not a replacement silhouette sampling approximation.
- Ellipsoid FOV visibility uses the artificial ellipsoid construction and ZZOCCED for circular/elliptical FOVs. Polygonal FOVs use EDLIMB, ZZELVUPY, INELPL, winding, and sector-intersection predicates. The former custom limb/cone sampling code was removed.
- Ray-direction oracle generation uses CSPICE MXV rather than hand-written matrix-vector sums, and comparisons preserve the original vector magnitude.

## Reference-path distinction

Several CSPICE native C vector/ellipse helpers and their internal Fortran counterparts use different floating-point operation order. TANGPT_C delegates to the internal Fortran TANGPT/NPEDLN path. The internal NPEDLN, EDLIMB, ZZOCCED, ZZASRYEL, ZZELVUPY, and INELPL fixtures therefore invoke the corresponding exported SPICELIB entry points through ctypes, matching the path of the calculation being repaired. This distinction is recorded in the generators; no precomputed geometry lookup is used by production code.

Source files are the official extracted N0067 distribution under `target/reference/cached-n0067/cspice/src/cspice`. Ephemeris fixtures use `de440s.bsp` and `pck00011.tpc`; the original surface fixtures use their documented `de442s.bsp` kernel. Orbital GM fixtures use `gm_de440.tpc`.

## Preserved NASA transport mismatch

`tests/golden/solar_longitude.csv` contains the original six WebGeocalc transport values. They have not been regenerated from local CSPICE. Their strict equality classification remains **FAIL**, with maximum difference **4.615856141754193e-9 degrees**. The regression named `reports_nasa_transport_solar_longitude_strict_mismatch` verifies and prints this audit classification; its passing test status does not mean the transport numbers match.

The cause of this transport difference has not been established here. The separate 60-case, same-kernel LSPCN comparison has zero difference. This transport result must remain separate from local numerical parity evidence.

## Reproduction

Generate the new independent references with:

```powershell
python tools/gen_surface_precision_golden.py
python tools/gen_surface_ray_precision_golden.py
python tools/gen_tangent_precision_golden.py
python tools/gen_orbit_precision_golden.py
python tools/gen_occult_ellipsoid_precision_golden.py
python tools/gen_fov_pyramid_precision_golden.py
python tools/gen_ray_direction_golden.py
```

Regenerate the original surface references with their original kernel selections:

```powershell
python tools/gen_surface_golden.py --spk target/kernels/de442s.bsp --pck target/kernels/pck00011.tpc
python tools/gen_illumination_golden.py --spk target/kernels/de442s.bsp --pck target/kernels/pck00011.tpc
python tools/gen_subpoint_method_golden.py
```

Run the numerical and geometric regressions with:

```powershell
cargo test --lib -- --nocapture
cargo test --test surface_precision --test tangent_precision --test orbit_precision --test gf_occultation_exact --test fov_ellipsoid_golden --test ray_direction_golden --test surface_golden --test subpoint_method_golden --test tangent_golden -- --nocapture
```

Kernel-dependent fixtures must actually execute. A skipped test because a kernel is absent is not exact-parity evidence. The workspace was supplied without a git repository; no git repository was initialized and no commit was created for these repairs.
