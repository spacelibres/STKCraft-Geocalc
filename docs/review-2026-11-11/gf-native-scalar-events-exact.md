# Native user-function and separation-event searches — 2026-11-11

The public `search_scalar` now executes the N0067 GFUDS solver with a native UDDC callback and a one-second UDDF abscissa step. `search_scalar_with_uddc` exposes that step. The UDDF/QDERIV implementation retains the source's `(0.5/dx)*f(t+dx) + (-0.5/dx)*f(t-dx)` ordering. This is the explicit GFUDS/UDDC contract; callers supplying an analytic decreasing predicate use `search_scalar_with_decreasing`. Callback evaluation must be defined outside confinement where the native solver and UDDF probe it.

An independent generator calls unmodified CSPICE GFUDS, UDDC and UDDF. Its 864 search cases cover eight functions, six positive/negative abscissa steps, single/multiple confinement segments, all seven relations, and absolute-extremum adjustment. Another 432 UDDF values and UDDC flags match exact bits/booleans. The old sampled solver failed 424 result comparisons; the source callback path has zero differences. The old custom scalar sampling, golden-section and bisection helpers were deleted.

`search_scalar_constraint` composes linear ranges using native serial searches. Periodic ranges return Unsupported and direct callers to `apply_scalar_constraint`, whose explicit coordinate domain supplies the missing branch information. The periodic margin proxy has been removed.

The legacy `search_separation` and `search_separation_shaped` now transform native GFSEP LOCMIN/LOCMAX/Equal windows into events. Crossing direction uses the same source ZZGFSPDC predicate as the native finder. No extra minimization, approximate epoch deduplication or scalar sampling changes the native roots. The event-only interface enforces isolated results. A separate constant-separation regression preserves N0067's empty LOCMIN/LOCMAX/Equal results, including an equality reference of zero for an identically-zero separation.

Independent GFSEP plus ZZGFSPGQ/ZZGFSPDC references cover 144 event cases: two observer/target configurations, all nine corrections, four POINT/SPHERE combinations, and optional threshold crossings. Every epoch, separation value and event direction matches exactly. The old implementation failed 180 result sets including the point-only convenience wrapper; the native implementation has zero differences. Original separation fixtures also now enforce exact equality for 156 scalar values and 14 event epochs; their former 1e-11 radian and 0.2-second acceptance bounds were removed.

Three original regression expectations were updated from mathematical roots to independently generated native GFUDS/UDDC results. This retains the source's time-refinement schedule, including its possible empty intersection with singleton confinement. The source values are recorded in `gf_uddc_regressions.csv`; no scalar tolerance band is introduced.

```powershell
python tools/gen_gf_uddc_precision_golden.py
python tools/gen_separation_native_events_precision_golden.py
cargo test --release --target-dir target/separation-agent --test gf_uddc_exact --test separation_native_events_exact --test finder_golden -- --nocapture
cargo test --release --target-dir target/separation-agent --lib native_uddf -- --nocapture
```

This evidence covers the listed contracts and source paths, rather than complete WebGeocalc parity. The native search step and time-convergence controls are inputs to the reference algorithm, and the tests require identical returned bits for those inputs.
