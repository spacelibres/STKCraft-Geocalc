# Dynamic frame strict comparison scope

All comparisons below use independently generated native CSPICE N0067 values. Float equality is exact; no tolerance is applied and no native case is ignored. PXFORM and SXFORM are tested separately, since their rotation blocks can differ in the last bit for adjacent Euler axes and two-vector constructions.

| Native fixture | Coverage | Cases |
| --- | --- | ---: |
| dynamic_euler.csv | All27axis sequences, degrees0/3/19,7angle units, rotating/inertial/frozen/default, inertial and body-fixed bases |12960|
| dynamic_two.csv | Signed different primary/secondary axes, rectangular scales1e-150..1e150, latitudinal and RA/DEC, mixedvector frames |6912|
| dynamic_product.csv |1/2/5/10factors, inverse-factor multiplication order, recursion rejection |1176|
| switch_frames.csv | Class6 priority, intervals/gaps/sharedboundaries, CKAVfallback, nested/dynamic/static/inertial bases |1728|
| dynamic_of_date.csv | Mean/true equator, mean ecliptic, IAU1976/1980, three rotation states, four bases |2592|
| dynamic_ephemeris.csv | Position, velocity and target-near-point vectors, all9corrections, constantS/XS/light-time cases, ET±3e9, inertial/body/dynamicvelocity frames |13680|

The ephemeris matrix includes13440valid transforms and240native INVALIDOPTION errors: light-time and stellar corrections cannot be combined for constant defining vectors. The product matrix includes24native RECURSIONTOODEEP errors. The public SXFORM API has a separate3240case check using the Euler fixture.

`Frames::attach_spk(Arc<SpkKernel>)` supplies ephemeris context. Clock/CK context is attached separately. Bothposition-onlySPKEZP andfullstateSPKEZR pipelines are preserved. Velocity-vector differentiation uses the exact native QDERIV inputs and the native asymmetric step `max(1,et*2^-27)`. This is the reference algorithm, not a freely chosen approximation.

`spkapo_dynamic.csv` separately checks1134position-only cases with dynamic source/outputframes and mixedframe/deepSPKchains. SourceSPKSSB simply delegates to SPKGEO inN0067, including its20node targetbuffer. The observerstate callback therefore uses that same source path.

The692built-in bodyaliases are extracted from official ZZIDMAP and independently checked by BODS2C. NAIF_BODY_NAME/CODE assignments and append forms are supported by the frame context; duplicate-name priority, blank compression and preferred-name OBJECT frame lookup have native regressions.

## Remaining limits

- Kernel dates use the exact registry Time 0.1.28 resolver, including the RSSD mixed-delimiter epoch. Eight raw native values and three rejection outcomes have pool regressions. Wider TPARSE grammar and date short-error symbols remain unverified; no approximate date substitution is made.
- Ordered general append and numeric-looking character values now have independent native regressions. See [kernel-pool scope](KERNEL_POOL_SCOPE.md) for exact coverage and residual limits.
- Dynamic metadata error/type/cardinality coverage is incomplete beyond the active native rejection cases listed above. Unverified options must not be reported as PASS.
- Native internal cache capacity/eviction limits have not been emulated; tested query-history semantics in the CK engine are preserved.

Source hashes are in DYNAMIC_FRAME_SOURCE_HASHES.json. Fixture generators are the correspondingly named tools/gen_dynamic_*.py and tools/gen_switch_frames.py.
