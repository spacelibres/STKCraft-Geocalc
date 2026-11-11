# Native RANGE and window processing evidence — 2026-11-11

`src/gf_constraints.rs` provides a callable RANGE composition over native geometry finders. Its callback receives the relation, reference and current confinement window. It performs no scalar sampling, periodic margin search or numerical derivative.

The independent CSPICE N0067 fixtures require identical endpoint bits in 504 native search combinations: all nine corrections, distance, latitudinal longitude, cylindrical longitude, right ascension, single/multiple confinement segments, normal/reversed/equal bounds, branch boundaries and multiple turns. Another 240 source WN combinations require exact window and duration bits, including singleton confinement segments and signed adjustment/filter parameters. Nine contract tests cover the scheduling and live API boundary behavior.

`finder::difference_windows` now delegates to the same source WNDIFD implementation. A separate 1,188-case native GFDIST/GFPOSC corpus for singleton confinement segments failed in 225 cases before this change and passes exactly afterwards. The earlier per-segment WNCOMD composition incorrectly retained covered singleton confinement intervals.

## Live contract evidence

The preserved probe files contain 41 completed NASA requests. Four requests intentionally use duplicate separation targets and are rejected. All 37 successful rows/windows match the independent reference when rendered with NASA's six-decimal time and eight-decimal number transport format. Under raw exact decimal comparison, 17 cases remain transport mismatches; the comparison report retains that failure classification. Rendering is diagnostic and does not relax exact acceptance.

The probes establish these distinctions:

- RANGE runs `> lower` first, then `< upper` inside the first result. Searching both bounds independently changes endpoints by several 1e-7 seconds. Three distance intervals' NASA durations distinguish the schedules.
- Reversed nonperiodic bounds are exchanged. Original equal bounds invoke `Equal`: the 400,000 km distance probe has three genuine singleton crossings, identical to the separate `=` request.
- A periodic arc crossing the native branch is the complement of the serial search of the opposite arc. Separate OR searches leave a time-tolerance-sized branch gap.
- Keep input angular bounds in degrees through scheduling; convert to radians inside the native callback. Converting 10 and 370 degrees first can reverse their normalized ordering through floating-point rounding. NASA returns an empty RANGE for these distinct equivalent-phase bounds, but a genuine singleton for 10..10.
- Signed longitude retains both -180 and +180 (`upper_inclusive=true`). Positive coordinates use [0,360) (`upper_inclusive=false`). Thus longitude -180..180 is full, longitude 180..-180 is empty, and right ascension/cylindrical longitude 0..360 is empty. Positive 540 retains the +180 side; -540 retains -180. Their branch endpoints differ at NASA's displayed precision.
- Postprocessing runs complement relative to the original multi-segment confinement, then adjustment, then filtering. Expansion may extend beyond confinement. Filtering removes intervals of duration **less than or equal to** its threshold. Negative expansion contracts, and negative filtering thresholds are accepted.

## Native use

For a linear search, call `apply_scalar_constraint` with a closure that invokes `find_distance`, `find_range_rate` or the appropriate native finder and **uses the closure's confinement argument**. For angular WGC inputs, use a 360-degree domain and multiply the closure reference by CSPICE RPD at the finder call. `ScalarConstraint::Range` handles ordinary scalars; `AngularRange` activates periodic arc scheduling.

Apply `postprocess_windows(result, original_confinement, parameters)` after the native search, retaining the original confinement separately. The interval epochs remain seconds. `window_durations` returns the normalized window's durations in the requested output units. Adjustment/filter amounts have their own explicit units.

The value-only scalar API now uses native GFUDS with UDDC(1 second). Its linear constraint wrapper uses the same serial composition; periodic value-only constraints return Unsupported because they lack the native coordinate-domain context. WGC geometry searches should use the native callback path described above, with the corresponding model's source derivative. This evidence does not establish complete WebGeocalc parity or raw equality with all NASA transport values.

```powershell
python tools/gen_gf_constraints_precision_golden.py
python tools/gen_gf_singleton_windows_precision_golden.py
cargo test --release --target-dir target/separation-agent --lib gf_constraints:: -- --nocapture
cargo test --release --target-dir target/separation-agent --test gf_constraints_exact -- --nocapture
python tools/compare_gf_constraints_live.py
```

The probe programs and complete requests/responses are named `tools/probe_gf_constraints*_wgc.py` and `docs/review-2026-11-11/gf-constraints*-live.json`. `gf-constraints-live-comparison.json` contains the strict transport classification and the separate rendering diagnostic. All data references the same DE442 small SPK and PCK00011 constants as the live kernel-set-1 queries.
