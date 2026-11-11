# Native DSK geometry checkpoint

This checkpoint adds a native Rust DAS/DLA and type-2 DSK reader and source-order DSK geometry. The tests use unmodified CSPICE N0067 as an independent oracle and compare floating-point bits directly. No tolerance is used for numerical acceptance.

## Implemented paths

- DAS logical character, double, and integer arrays; linked physical directories; reserved/comment records; little-endian and big-endian IEEE files.
- DLA segment traversal and full DSK descriptors, vertices, plates, and spatial indices.
- DSKX02 ray intersections using ZZRAYBOX, XDDA voxel traversal, native plate-list ordering and duplicate retry, PLTEXP, INSANG, facing tests, and segment bounds.
- Unprioritized DSKXSI unions with source segment traversal, surface-ID and coverage-time filters, frame transforms, volume-candidate ordering, and the source early termination schedule.
- DSK point-to-plate membership and surface normals through ZZPTPL02, PLTNP, and ZZDSKBUN.
- SINCPT, SUBPNT NADIR/INTERCEPT, SUBSLR NADIR/INTERCEPT, and ILLUMF through explicit DSK APIs. NADIR uses the reference ellipsoid only to define a direction, then intersects the actual DSK surface.
- Dynamic FIXREF calculations distinguish the source PXFORM geometry rotations from the SXFORM rotations used by SPKCPO/SPKCPT state outputs.

The new public geometry types are `DskKernel`, `DskSet`, `DskSegment`, `DskDescriptor`, `DlaDescriptor`, `DskHit`, `SurfaceModel`, and `DskSubPointMethod`. Explicit model functions are defined in `src/geometry/surface.rs`; existing ellipsoid wrappers retain their signatures.

## Independent reference coverage

| Check | Native cases | Result |
| --- | ---: | --- |
| Descriptor, vertex, plate, spatial metadata and normals | 112 rows | Exact |
| DAS layouts, including four linked directories, reserved records and validated big-endian files | 2,004 segments | Exact |
| DSKX02 sparse and concave 512-plate models, clipped LAT/REC/PDT domains, edge rays and remote observers | 12,290 rays | Exact |
| ZZINVELT base predicates | 5,600 predicates | Exact |
| ZZINVELT excluded coordinates and zero/greedy margins | 8,400 predicates | Exact |
| ZZSEGBOX | 180 descriptors / 720 components | Exact |
| ZZRYTELT ordinary and boundary cases | 6,336 rays | Exact |
| INCNSG | 4,200 cone/segment cases | Exact |
| ZZCNQUAD | 6,412 quadratic cases | Exact |
| ZZPTPL02 | 11,502 cases: 4,506 found and 6,996 misses | Exact |
| PLTNP | 2,400 cases / 9,600 components | Exact |
| DSKXSI unions | 8,640 cases: 2,876 hits, 4,804 misses, 960 native rejections | Exact |
| SINCPT/SUBPNT DSK | 2,160 cases: 1,444 numeric results, 176 misses, 540 native rejections | Exact |
| Dynamic FIXREF ellipsoid/DSK SINCPT, SUBPNT and SUBSLR | 2,160 cases: 1,776 numeric results and 384 native transmission rejections | Exact |
| ILLUMF DSK | 10,368 cases: 7,968 numeric results and 2,400 native off-surface rejections | Exact |

For expected native errors, the tests require an error outcome. They do not assert identical SPICE error-message text. Corrupt-file tests additionally check rejection of truncated records, invalid directory links and oversized cluster descriptors.

The synthetic DSKs are written with native DSKMI2/DSKW02. Layout transformations are reopened and validated by native CSPICE before their oracle rows are saved. The generators are `tools/gen_dsk_*precision_golden.py` plus the independently authored volume and point generators. Fixtures are under `tests/golden/dsk/`.

## Source behaviors retained

N0067 DSKXSI updates its shared integer source-info output for each tested hit, but does not restore that integer when an earlier hit wins. Consequently, its returned plate ID can differ from the plate that supplied the returned point and descriptor. `DskHit.plate_id` preserves the native returned integer; `geometric_plate_id` identifies the actual winning plate used for its normal. The strict oracle compares the native plate-ID output without substituting the geometric ID.

N0067 ILLUMF parses a surface list and then sets the normal/occlusion callback surface count to zero. The new ILLUMF path preserves that behavior, including a requested surface ID absent from the loaded kernels. Its surface-point construction is a separate concern.

Prioritized DSK methods are rejected by N0067 ZZPRSMET. The implemented public model therefore explicitly uses unprioritized DSK data. Type-2 geometry uses N0067's default DSK tolerances; these are source algorithm parameters, not acceptance tolerances.

## Remaining integration scope

This checkpoint does not complete every WebGeocalc DSK workflow. The generic state `Endpoint` variants and existing high-level ray-result facades still require explicit DSK model integration. Surface-name-to-ID kernel-pool resolution and the WebGeocalc coordinate-to-DSK-point facade are not added here. DSK-versus-point occultation search also remains separate work. Unsupported DSK data types are rejected explicitly.

The independent native fixtures prove the exercised calculations and kernels. They do not establish universal parity with every NASA server kernel set or every input. Previously recorded NASA decimal-transport mismatches remain strict failures in their separate reports.

## Verification

At the verified release checkpoint, all 109 library tests and the DSK, surface precision, explicit target-frame, rotating-SPK, typed DREF, legacy ray and surface integration tests passed: 132 tests in this selected run, with zero failures, ignored tests or skips. The legacy NASA solar-longitude transport diagnostic continues to report its preserved strict mismatch; it is not relabeled as numeric agreement.

Official source hashes and source-port notes are recorded in `docs/reference/DSK_SOURCE_HASHES.json`.
