# Surface-intercept direction frames — 2026-11-11

`find_surface_intercept_ray_coordinate_in` accepts a fixed vector and its named DREF, with a `Frames` context. The original J2000 entry point delegates to it. `find_surface_intercept_direction_coordinate` accepts the four fixed direction forms documented for GF_SURFACE_INTERCEPT_POINT_SEARCH: instrument boresight, reference-frame axis, instrument-frame vector and reference-frame vector. It retains the vector in its source frame through the native state calculation. The GF facade rejects object directions, boundary-vector expansion and an additional stellar correction on the input vector, which are absent from that calculation's direction schema.

The source paths deliberately use separate frame-epoch calculations:

- SINCPT position uses SPKEZP light time to a non-inertial direction frame's center. Inertial or observer-centered direction frames use ET.
- ZZGFSSIN direction state uses SPKACS light time and its rate. Its frame derivative is corrected by ZZCORSXF, then the complete direction state is transformed into the target frame for SURFPV. The surface intercept velocity is analytic; no sampled derivative replaces this source path.

Independent native CSPICE N0067/SpiceyPy 8.3.0 results require exact output bits:

| Corpus | Cases | Checked outputs |
|---|---:|---|
| Direct Fortran ZZGFSSIN | 1,080 | FOUND flags and 5,832 state components for 972 found states |
| GFSNTC named direction frames | 4,536 | Complete windows, including 2,061 nonempty intervals |
| Four GF direction specifications | 756 | Complete native GFSNTC windows |
| Typed SINCPT/TANGPT DREF flows | 540 | Resolved vectors, intercept presence/point/range and all 12 native tangent outputs |

The named-frame corpora cover Earth and Moon as targets, all nine corrections, epochs -5e8/0/+5e8, J2000/ECLIPJ2000/IAU_EARTH and a TKFRAME relative to IAU_MOON. The typed ray corpus also includes IAU_MOON and reversed vectors. GF windows cover rectangular X, latitudinal longitude and planetographic latitude, with all seven native relations.

The high-level `resolve_ray_directions_with_pck` previously sampled every direction frame at ET. Its 540-case regression initially had 345 differing output groups. Applying the native DREF epoch removed all vector and tangent differences; 67 remaining intercept range differences exposed a separate metadata issue. Public surface-intercept distance now uses VNORM of the returned surface vector, eliminating those one-ULP differences without changing points, epochs or vectors. All 540 typed cases then pass exactly. Existing ray, surface, subpoint and explicit target-frame suites also pass.

Reproduction:

```powershell
python tools/gen_gf_intercept_dref_precision_golden.py
python tools/gen_gf_intercept_specs_precision_golden.py
python tools/gen_ray_dref_precision_golden.py
cargo test --release --target-dir target/separation-agent --lib intercept_dref_states -- --nocapture
cargo test --release --target-dir target/separation-agent --test gf_intercept_dref_exact --test gf_intercept_specs_exact --test ray_dref_exact -- --nocapture
```

Explicit target-frame backends now use the selected FIXREF throughout geometry and analytic state calculations. `find_surface_intercept_ray_coordinate_with_frame` and `find_surface_intercept_direction_coordinate_in` retain IAU wrappers for existing callers. A further 4,536 GFSNTC cases cover MOON_ME/MOON_PA/TKFRAME with binary lunar orientation, both native and synthetic triaxial radii: 3,780 native numeric searches and 756 expected unsupported planetographic searches. All windows and 648 additional direct ZZGFSSIN six-states are exact.

`find_sub_point_coordinate_in` and `find_illumination_angle_in` extend the same frame handling to GFSUBC and GFILUM. Each has 972 native oracle cases, including source rejections; 324 direct ZZGFSSOB six-states also match exactly. Both subpoint methods, rectangular/latitudinal/planetographic coordinates, Sun/Venus illumination, and all nine correction requests are covered. GFILUM's four transmission corrections are rejected like the native routine. GF geodetic/planetographic searches reject unequal equatorial radii at initialization, including empty confinement.

These comparisons establish the listed source paths, not complete WebGeocalc parity. No numeric acceptance tolerance is used here.
