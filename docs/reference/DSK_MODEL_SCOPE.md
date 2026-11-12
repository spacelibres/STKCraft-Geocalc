# Explicit surface-model integration

This is a representative native acceptance record, not complete WebGeocalc
conformance. Production remains pure Rust. The independent processes link the
unmodified official N0067 static library using `/Od /fp:strict /MT`, the pinned
static UCRT and its unchanged default FMA setting. Raw captures, source
snapshots, compiler output, linker response files and hashes are retained in
[the review evidence](https://github.com/spacelibres/STKCraft-Geocalc/tree/v0.1.10/docs/review-2026-11-12/).

## Coordinate construction

`latitudinal_surface_points_in` implements LATSRF with an explicit
`SurfaceModel`, epoch, target-centered frame and numeric surface IDs. Inputs
are planetocentric longitude/latitude pairs in radians. Every point must
exist. Native POINTNOTFOUND and unsupported opposing-surface outcomes remain
errors; they are not replaced by ellipsoid points.

The independent capture contains 2,352 calls: 1,470 three-component results
and 882 errors. It exercises two type-2 meshes, two frames, three epochs,
four model/filter choices and a seven-by-seven angle grid, including poles
and longitudes outside one revolution. Every result component is compared
by binary64 representation. See
[the original capture](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.10/docs/review-2026-11-12/surface-coordinates/capture.json).

WebGeocalc's `SurfacePointSpecification` uses on-surface LATITUDINAL or
SPHERICAL coordinates for DSK illumination points. Its separate
`LocationSpecification` does not introduce DSK shapes for STATE_VECTOR
endpoints. These are distinct contracts in the
[official API description](https://wgc2.jpl.nasa.gov:8443/webgeocalc/documents/api-info.html),
preserved in [the original page capture](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.8/docs/review-2026-11-11/official-api-info.html).
This change does not invent a DSK point-location endpoint.

## Ray intersections and all result fields

`surface_intercept_geometries_with_model_in` accepts the existing typed
`RayDirection` and an explicit shape model. It returns one `Option` per
resolved ray, retaining order. `None` represents the native missing-intercept
flag. Other failures remain errors. Existing public entry points are retained.

The SINCPT capture has 1,296 calls: 850 results, 122 misses and 324 errors.
It covers ellipsoid and DSK/all/100/999 filters, nine correction modes,
two target frames, Earth/Sun observers, three epochs and three ray offsets.
Both the direct and typed `VectorInFrame` paths check all records: 2,592
executions, 1,700 results, 244 misses and 648 errors.

Each successful record compares the three intercept coordinates, target
epoch, three observer-to-surface components, native VNORM distance and
VNORM/CLIGHT light time: nine unrounded cells. The last quantity is derived
independently in C from the returned SINCPT vector. It does not claim to
observe an unexposed, unrounded NASA server field.

Before repair, 198 light-time fields differed because Rust exposed the
internal iteration estimate instead of distance divided by the speed of
light. Another 324 records returned a generic container message for absent
surface data. The repair preserves the intercept and epoch algorithms,
computes the reported light time from the returned vector, and returns the
native DSKDATANOTFOUND symbol. Original expectations are unchanged. See
[the original capture](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.10/docs/review-2026-11-12/surface-intercepts/capture.json)
and the retained before/after logs in the same directory.

## DSK versus point occultation searches

`find_occultation_with_models_in` selects `OccultationModel::Point` or
`OccultationModel::Surface { frame, model }` for each target. A DSK requires
the other target to be a point. Any point requires occultation type ANY.
Native initialization, radial bounds, depth tests, stellar-correction
stripping and SINCPT callbacks feed the existing native boolean-search
schedule. A search convergence tolerance is an input, not a comparison band.

The independent GFOCLT capture contains 864 queries: 144 successful windows
(72 nonempty) and 720 errors. It exercises two point depths, both front/back
assignments, nine corrections, two DSK frames, three surface filters and
all four occultation conditions. Start, stop and duration bits, empty
windows and error symbols all match. The original 432-query back-point
capture was also compared unchanged when the foreground-point cases were
added. See [the capture](https://github.com/spacelibres/STKCraft-Geocalc/blob/v0.1.10/docs/review-2026-11-12/dsk-occultation/capture.json).
The new explicit-model entry point additionally checks all 162 existing
independent ellipsoid/point GFOCLT window records without replacing them.

## Boundaries still open

- Surface names and native surface-name cache history/capacity semantics.
- The full product of typed ray sources, FOV multi-ray options and models;
  the new typed regression specifically exercises `VectorInFrame`.
- Shape-aware illumination/subpoint facades and CLI/MCP exposure of these
  new Rust entry points. Existing tools retain their documented scope.
- Exhaustive frame/radii metadata types and cardinalities, empty batches,
  degenerate geometry, combined-error precedence and malformed DSKs.
- Arbitrary explicitly framed ellipsoid-pair occultation matrices; the
  retained 162-record regression uses its original IAU frame inputs.
- Other runtime profiles and exhaustive WebGeocalc output/option products.

The strict gate requires all four new regression names in both Debug and
Release. It must pass against the final source inventory before publication.
Complete WebGeocalc parity remains false.
