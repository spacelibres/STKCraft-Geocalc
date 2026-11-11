# API guide

Use the generated API reference with cargo doc --open. The crate uses std and forbids unsafe Rust. It is not a no_std implementation.

## Geometric and apparent states

```rust,no_run
use stkcraft_geocalc::{Aberration, SpkKernel};

let kernel = SpkKernel::open("de442.bsp")?;
let geometric = kernel.state(301, 399, 0.0)?;
let apparent = kernel.state_corrected(301, 399, 0.0, Aberration::ConvergedLightTimeStellar)?;
assert!(geometric.position.iter().all(|v| v.is_finite()));
assert!(apparent.light_time > 0.0);
# Ok::<(), stkcraft_geocalc::GeoError>(())
```

Positions are kilometres; velocities are kilometres per second. Input et is TDB seconds past J2000, not UTC, Unix time or a Julian Date. Convert calendar times through the distinct STKCraft-time NAIF contract.

SpkKernel::open_all preserves file load order. Later applicable files/segments take priority. Type-10 TLE segments require attach_lsk. Context-free methods support built-in inertial segment references; rotating source references require the explicit Frames/PCK methods.

## Frames and the position/state distinction

Use Frames::open or furnsh for FK definitions, Pck::open_all for text/binary PCK, attach_ck for attitudes, attach_sclk for spacecraft-clock context, and attach_spk(Arc<SpkKernel>) for ephemeris-defined frames.

- position_in_frame follows SPKGPS/REFCHG.
- position_corrected_in_frame follows SPKEZP/SPKAPO and returns position plus light time.
- state_in_frame follows SPKGEO/FRMCHG.
- apparent_state_in with FrameSource::Named follows SPKEZR and returns the full state and correction rates.
- Frames::pxform_in is rotation-only. pxform_with_derivative supplies a state transform. Their rotation blocks can differ by a binary64 interval for dynamic definitions; do not interchange them.

## Shapes and surfaces

Pck supplies radii and orientation constants. The surface module exposes sub_observer_geometry_in, sub_solar_geometry_in, surface_intercept_geometry_in and illumination_geometry_in for ellipsoids.

For type-2 DSK, load a DskSet in file order and select SurfaceModel::DskUnprioritized with explicit numeric surface IDs. Use surface_intercept_geometry_with_model_in, sub_observer_dsk_geometry_in, sub_solar_dsk_geometry_in and illumination_geometry_with_model_in. DskSubPointMethod::Nadir and ::Intercept have distinct native meanings; an ellipsoid NearPoint option is not renamed into DSK Nadir.

DskHit::plate_id preserves the native aggregate IC output; geometric_plate_id identifies the plate that generated the winning intercept. See the [DSK evidence](review-2026-11-11/dsk-exact.md) for the native metadata quirk.

## Search and other calculations

finder and gf_constraints implement scalar relations, RANGE scheduling, confinement and interval processing. Supply the same search step, convergence inputs and windows used by the native reference. Other public modules provide orbital elements, pointing, coordinate representations, FOV, occultation, tangent points and point-location states.

Read [the remaining scope](STRICT_REMAINING_GAPS.md) before choosing a shape or frame combination. Unsupported inputs return GeoError; missing data is not replaced with an approximate model.
