<p align="center">
  <img src="https://raw.githubusercontent.com/spacelibres/STKCraft_web/main/public/brand/albatross.png" alt="STKCraft — Albatross" width="128">
</p>

# STKCraft-geocalc

**STKCraft project:** <https://www.stkcraft.com>
**Repository:** <https://github.com/spacelibres/STKCraft-Geocalc>

[![CI](https://github.com/spacelibres/STKCraft-Geocalc/actions/workflows/ci.yml/badge.svg)](https://github.com/spacelibres/STKCraft-Geocalc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/stkcraft-geocalc.svg)](https://crates.io/crates/stkcraft-geocalc)
[![docs.rs](https://docs.rs/stkcraft-geocalc/badge.svg)](https://docs.rs/stkcraft-geocalc)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

- [API guide](docs/API.md) and [generated reference](https://docs.rs/stkcraft-geocalc)
- [MCP server](docs/MCP.md) and [Agent Skills](.agents/skills)
- [Source architecture](docs/ARCHITECTURE.md), [numerical contract](docs/NUMERICS.md), and [validation](docs/VALIDATION.md)
- [Current evidence](docs/STRICT_PARITY_STATUS.md), [remaining gaps](docs/STRICT_REMAINING_GAPS.md), and [changelog](CHANGELOG.md)

## Installation and status

Version 0.1.6 is a versioned implementation checkpoint. Complete WebGeocalc
option-level conformance remains open; see the gap register above.
The crate requires Rust 1.75 or newer and uses `std`.

```sh
cargo add stkcraft-geocalc
cargo install stkcraft-geocalc --features mcp --bins
```

```toml
[dependencies]
stkcraft-geocalc = "0.1.6"
```

The optional `mcp` feature installs `stkcraft-geocalc-mcp` alongside
`wgc_geometry` and `wgc_gf`. Agent Skills are included in the Git repository,
crate archive and release distribution. See the MCP guide for configuration.
Time is pinned to the published `stkcraft-time` 0.1.18 dependency.

A NAIF/SPICE-aligned geometry library written in Rust. It reads binary
SPK kernels, evaluates geometric and aberration-corrected state vectors,
and evaluates fixed-rotation frame transforms defined by frame kernels,
matching CSPICE.

## Scope

Implemented:

- DAF container parsing (both byte orders).
- All16 SPK types dispatched by N0067: 1/2/3/5/8/9/10/12/13/14/15/17/18/19/20/21.
  Type10 requires an explicitly attached LSK from STKCraft-time.
- Segment chains, so `state(target, observer, et)` works when the kernel
  stores bodies relative to a common center.
- `state_at_epoch`, using `stkcraft-time` to convert TAI to ET.
- `apparent_state_in`, which resolves an output frame the way CSPICE
  `spkezr` does: a non-inertial frame is evaluated at its center's corrected epoch
  and the body-fixed velocity carries the `1 + sigma*dlt/dt` term. See
  [Frame semantics](#frame-semantics).
- All eight WebGeocalc state representations (`RECTANGULAR`, `RA_DEC`,
  `LATITUDINAL`, `CYLINDRICAL`, `SPHERICAL`, `AZ_EL` with both flags,
  `PLANETODETIC`, `PLANETOGRAPHIC`) with the exact WGC column order and
  analytic coordinate rates (`src/geometry/state_rep.rs`).
- Frame kernels: fixed-rotation `TKFRAME` definitions (`MATRIX`,
  `ANGLES` with every NAIF angle unit, and `QUATERNION`), relative-frame
  chains, and all 21 built-in inertial frames with 145 built-in frame definitions.
- `pxform` and `sxform`, resolved through the lowest common ancestor of
  the two frames.
- Binary PCK types2/3/20, all six CK types with SCLK linkage, and dynamic
  Euler, two-vector, product, Earth of-date and switch frames.
  Independent native fixtures exercise records, state
  derivatives, coverage, load priority and frame composition.
- `pxform_with_derivative`, the rotation between two frames together with
  its **analytic** time derivative. It composes `TKFRAME` chains and
  PCK body-fixed frames (named `IAU_<naif-id>`) with the product rule, so
  the rate comes from the IAU orientation model rather than a difference
  quotient. Verified against CSPICE `sxform` over fourteen frame pairs.
- All seven WebGeocalc orientation representations (`EULER_ANGLES`,
  `ANGLE_AND_AXIS`, `SPICE_QUATERNION`, `OTHER_QUATERNION`,
  `MATRIX_ROW_BY_ROW`, `MATRIX_FLAGGED`, `MATRIX_ALL_ONE_ROW`) and all
  five angular velocity representations (`NOT_INCLUDED`,
  `VECTOR_IN_FRAME1`, `VECTOR_IN_FRAME2`, `EULER_ANGLE_DERIVATIVES`,
  `MATRIX`) with the live WGC row and column layout. See
  [Frame transformation output](#frame-transformation-output).
- Point locations: `FIXED_POINT` and `CONSTANT_VELOCITY_POINT` targets
  and observers, in all eight WebGeocalc coordinate systems
  (`RECTANGULAR`, `RA_DEC`, `LATITUDINAL`, `PLANETODETIC`,
  `PLANETOGRAPHIC`, `CYLINDRICAL`, `SPHERICAL`, `AZ_EL`), with the
  rotating-frame velocity term and light-time iteration matching CSPICE
  `spkcpo`, `spkcvo`, `spkcpt`, and `spkcvt`. See
  [Point locations](#point-locations).
- The local topocentric frame (`TOPOCENTRIC_AT_POINT`), `+Z` along the
  surface normal at the observer's nearest point on the ellipsoid and
  `+X` towards the body's north pole, matching `azlcpo`.
- All three `frameLocus` choices (`CENTER`, `OBSERVER`, `TARGET`), which
  differ only when the output frame rotates.
- Light-time and stellar-aberration corrections matching CSPICE
  `spkezr`: `NONE`, `LT`, `LT+S`, `CN`, `CN+S`, and the
  `X`-prefixed transmission variants.
- Angular separation between two targets matching CSPICE `trgsep` with
  `POINT` and `SPHERE` shapes. A sphere's radius is the largest PCK
  radius, the angle is measured between the apparent limbs, and the
  result is negative when the disks overlap, exactly as `trgsep`
  documents. Plus a time-domain search for local extrema and threshold
  crossings with either shape.
- Phase angle matching CSPICE `phaseq`, and angular size matching the
  WebGeocalc Angular Size calculation with `MAX_RADIUS` (its default),
  `MIN_RADIUS`, and `LIMB_EXTENTS`.
- Text PCK parsing: body radii, gravitational parameters, and the IAU
  body-fixed orientation model (`POLE_RA`, `POLE_DEC`, `PM`, and the
  `NUT_PREC_*` series, including the `MAX_PHASE_DEGREE` quadratic phases),
  checked against CSPICE `pxform` and `sxform` for eleven bodies.
- `Pck::body_state_rotation`, the IAU rotation matrix together with its
  analytic time derivative, checked entry by entry against CSPICE `sxform`.
- The planetographic longitude direction follows the body's spin sense,
  taken from `BODY<id>_PGR_POSITIVE_LON` or the first-degree
  prime-meridian coefficient, so retrograde bodies such as Venus and
  Uranus are correct rather than assumed west-positive.
- Osculating orbital elements matching CSPICE `oscelt`.
- Sub-observer and sub-solar points (near point or centre-ray intercept
  on the IAU ellipsoid), matching CSPICE `subpnt` and `subslr`.
- Surface intercept points (`sincpt`, including arbitrary fixed rays),
  illumination angles (`ilumin`), and planetocentric solar longitude.
- Position, velocity and explicit-vector pointing, original or unit magnitude,
  all six output coordinate representations, and instrument boundary rows.
- Ray-ellipsoid tangent points matching CSPICE `tangpt`, including the
  light-time and stellar-aberration iteration and the look-away case.
- Every WebGeocalc ray-direction form for surface intercepts and tangent
  points: `DIRECTION_TO_OBJECT`, `INSTRUMENT_BORESIGHT`,
  `INSTRUMENT_FOV_BOUNDARY_VECTORS`, `REFERENCE_FRAME_AXIS`,
  `VECTOR_IN_REFERENCE_FRAME`, and `VECTOR_IN_INSTRUMENT_FOV`, with
  `antiVectorFlag` and `vectorAbCorr`.
- Sub-observer and sub-solar points for both ellipsoid definitions
  (`NEAR POINT` and `INTERCEPT`).
- A source-order GF engine (`search_scalar_with_decreasing`) with CSPICE
  `RELATE` conditions: `>`, `<`, `=`, `LOCMIN`, `LOCMAX`, `ABSMIN`,
  `ABSMAX`, and adjusted absolute extrema.
- Geometry Finder searches built on that engine: distance (`gfdist`),
  range rate (`gfrr`), phase angle (`gfpa`), illumination angles
  (`gfilum`), position coordinate (`gfposc`), sub-point coordinate
  (`gfsubc`), and surface-intercept coordinate (`gfsntc`).
- Coordinate systems used by those searches: rectangular, latitudinal,
  RA/Dec, spherical, cylindrical, geodetic, and planetographic
  (including the NAIF east/west longitude convention).
- Instrument FOV parsing (`INS<id>_FOV_*`, `CORNERS` and `ANGLES`
  classes) and point/ray visibility matching `fovtrg`/`fovray` for
  `CIRCLE`, `ELLIPSE`, `RECTANGLE`, and `POLYGON` shapes.
- Ellipsoid-target FOV visibility matching `fovtrg` with
  `TSHAPE = 'ELLIPSOID'` for all four FOV shapes. The target's apparent
  visibility predicate follows the N0067 analytic ZZOCCED/ZZELVUPY paths;
  no replacement curve-sampling approximation is used.
- Occultation states matching `occult` and interval searches matching
  `gfoclt` for triaxial ellipsoids and point shapes. The classification
  uses the exact apparent-limb conics, not a spherical approximation.
- Type-2 DSK/DAS/DLA reading and unprioritized multi-segment intercepts,
  with direct DSK SINCPT, SUBPNT, SUBSLR and ILLUMF APIs. Explicit shape
  selection preserves source surface/filter semantics and errors.

Not implemented yet (stated, not approximated):

- DSK integration into point-location and ray/coordinate facades,
  surface-name lookup, and DSK-versus-point occultation search.
- Kernel date literals and additional dynamic/kernel-pool options are tracked in
  [the option audit](docs/STRICT_REMAINING_GAPS.md). Supported classes
  have independent exact fixtures; this does not certify every option.

## Usage

```rust
use stkcraft_geocalc::SpkKernel;

let kernel = SpkKernel::open("de440s.bsp")?;
// Geometric state of Earth relative to the solar system barycenter.
let state = kernel.state(399, 0, 0.0)?;
println!("{:?} km, {:?} km/s", state.position, state.velocity);
```

`et` is TDB seconds past J2000, exactly as in SPICE.

```rust
use stkcraft_geocalc::{Aberration, SpkKernel};

let kernel = SpkKernel::open("de440s.bsp")?;
// Apparent state of the Moon as seen from Earth, corrected for light
// time and stellar aberration.
let apparent = kernel.state_corrected(301, 399, 0.0, Aberration::LightTimeStellar)?;
println!("{:?} km", apparent.state.position);
println!("light time {:.6} s", apparent.light_time);
```

`state_corrected` evaluates the target and observer relative to the
solar system barycenter, which is the same requirement CSPICE has for
stellar-aberration corrections.

### Frame semantics

`apparent_state_in` is the `spkezr` equivalent for a frame that is not
necessarily inertial:

```rust
use stkcraft_geocalc::{Aberration, FrameSource, Pck, SpkKernel};

let kernel = SpkKernel::open("de442s.bsp")?;
let pck = Pck::open("pck00011.tpc")?;

// EARTH as seen from the Moon, in the Moon's body-fixed frame.
let frame = FrameSource::BodyFixed { pck: &pck, body: 301 };
let apparent = kernel.apparent_state_in(
    399, 301, 0.0, Aberration::LightTimeStellar, &frame,
)?;
```

CSPICE evaluates a non-inertial output frame at the epoch at which the
**observer** sees the **frame's centre**, `et + sigma*lt(observer,
centre)`, with `sigma` equal to `-1` for reception corrections and `+1`
for the `X`-prefixed transmission ones. When the observer is the centre
the shift is zero. Because that epoch moves with the observer, the
body-fixed velocity is `M v + (1 + sigma*dlt/dt) M' r`. Rounding the last
factor to one leaves a few micrometres per second of velocity error in
the Earth-Moon system, which is why it is modelled.

Note that this is *not* the observer-target light time. The two coincide
only when the target is the frame's centre, which is the usual case but
not the general one; using the wrong one rotates the result about the
frame's spin axis by the difference (`215 km` in the case that exposed
it). The rule was established by comparing against CSPICE `spkezp` with
the target and the frame centre deliberately chosen to differ, not
assumed.

`FrameSource::Fixed` represents a constant inertial matrix, such as
`ECLIPJ2000`. `FixedNonInertial` separately represents a constant TKFRAME
with its center. CSPICE applies aberration corrections differently for
class-1 inertial and class-4 fixed frames, even when their matrices coincide.

### Frame transformation output

`describe` turns a rotation and its derivative into the WebGeocalc
`FRAME_TRANSFORMATION` columns. The shapes below were measured against
the live service rather than inferred:

| Orientation | Rows | Columns (after the date) |
|---|---|---|
| `EULER_ANGLES` | 1 | `ANGLE3, ANGLE2, ANGLE1` |
| `ANGLE_AND_AXIS` | 1 | `ANGLE, AXIS_X, AXIS_Y, AXIS_Z` |
| `SPICE_QUATERNION` | 1 | `QCOS, QSIN1, QSIN2, QSIN3` |
| `OTHER_QUATERNION` | 1 | `QSIN1, QSIN2, QSIN3, QCOS` (vector part negated) |
| `MATRIX_ROW_BY_ROW` | 3 | `ROW_NUMBER` plus three entries per row |
| `MATRIX_FLAGGED` | 3 | three entries per row, no row number |
| `MATRIX_ALL_ONE_ROW` | 1 | nine entries in one row |

Two rules are easy to get wrong and were confirmed live:

* An angular velocity vector adds `AV_X, AV_Y, AV_Z, AV_MAG` to **every**
  row of the multi-row matrix forms, not just the first one.
* `MATRIX` angular velocity replaces the 3x3 block with the full 6x6 state
  transformation (`[R 0; R_dot R]`): six rows of six for the row-by-row
  and flagged forms, one row of 36 for `MATRIX_ALL_ONE_ROW`. For the angle
  and quaternion forms the service drops it entirely and returns the
  orientation columns alone.

`EULER_ANGLE_DERIVATIVES` is accepted only with `EULER_ANGLES` and the
same axis sequence; anything else is rejected rather than silently
reinterpreted, which is what the service also does.

```rust
use stkcraft_geocalc::{
    describe, AngularUnits, AngularVelocityRepresentation, Frames, Pck,
    OrientationRepresentation,
};

let frames = Frames::new();
let pck = Pck::open("pck00011.tpc")?;
let rotation = frames.pxform_with_derivative("J2000", "IAU_301", 0.0, Some(&pck))?;
let columns = describe(
    &rotation.rotation,
    &rotation.rotation_dot,
    OrientationRepresentation::SpiceQuaternion,
    AngularVelocityRepresentation::VectorInFrame1,
    AngularUnits::Degrees,
)?;
println!("{:?}", columns.ids);
```

### Point locations

WebGeocalc can target or observe from a location that has no ephemeris
object. `PointLocation` covers both forms:

```rust
use stkcraft_geocalc::{
    express, state_between, Endpoint, FrameLocus, Frames, LocationCoordinates,
    OutputFrame, Pck, PointLocation, SpkKernel,
};

let kernel = SpkKernel::open("de442s.bsp")?;
let pck = Pck::open("pck00011.tpc")?;
let frames = Frames::new();

// A station 0.3 km above the reference spheroid of the Earth.
let station = PointLocation::from_coordinates(
    399,
    "IAU_399",
    LocationCoordinates::Planetodetic {
        longitude: -118f64.to_radians(),
        latitude: 34f64.to_radians(),
        altitude: 0.3,
    },
    &pck,
)?;

let et = 1.0e8;
let observer = Endpoint::Point(&station);
let corrected = state_between(
    &kernel, &frames, &pck, Endpoint::Object(301), observer, et,
    stkcraft_geocalc::Aberration::LightTimeStellar,
)?;
let apparent = express(
    &kernel, &frames, &pck, corrected, observer,
    OutputFrame::Topocentric(&station), None,
    FrameLocus::Center, et,
    stkcraft_geocalc::Aberration::LightTimeStellar,
)?;
```

Three details are implemented rather than left to the caller, because
each of them changes the last figures:

* The coordinates are given in a frame that may rotate, so the point's
  J2000 velocity is `R v_frame + R_dot r_frame`; a station on the Earth
  moves with the Earth.
* A `CONSTANT_VELOCITY_POINT` is extrapolated linearly from its epoch, and
  the light-time iteration samples that extrapolation at the retarded
  epoch, as `spkcvo` and `spkcvt` do.
* The topocentric frame's `+Z` is the outward normal at the observer's
  **nearest point** on the ellipsoid, not the gradient at the observer's
  altitude. For an oblate body the two differ, and `azlcpo` uses the
  nearest point.

Exactly one endpoint of an observation may be a point; WebGeocalc
enforces the same restriction.

### State representations

`state_columns` produces the WebGeocalc `STATE_VECTOR` column set in the
order WGC emits it, with angles in radians and angular rates in
radians per second. `StateColumns::numeric_row` converts to the degrees
and degrees per second that WGC prints.

```rust
use stkcraft_geocalc::{Aberration, Pck, SpkKernel, StateRepresentation};

let kernel = SpkKernel::open("de442s.bsp")?;
let pck = Pck::open("pck00011.tpc")?;
let frame = stkcraft_geocalc::FrameSource::BodyFixed { pck: &pck, body: 301 };
let apparent = kernel.apparent_state_in(
    301, 399, 0.0, Aberration::LightTimeStellar, &frame,
)?;

let columns = stkcraft_geocalc::state_columns_with_pck(
    0.0,
    apparent.state,
    StateRepresentation::Planetographic,
    apparent.light_time,
    Aberration::LightTimeStellar,
    &pck,
    301,
)?;
println!("{:?}", columns.numeric_row());
```

The rate columns are analytic derivatives of the coordinate map, not
finite differences. For the ellipsoidal representations they come from
differentiating the planetodetic back-substitution, which yields a 2x2
linear system for `(dlatitude/dt, daltitude/dt)`.

```rust
use stkcraft_geocalc::{
    angular_separation, search_separation, Aberration, SearchOptions, SpkKernel,
};

let kernel = SpkKernel::open("de440s.bsp")?;
let angle = angular_separation(&kernel, 399, 10, 301, 0.0, Aberration::None)?;
let events = search_separation(
    &kernel, 399, 10, 301, Aberration::None, 0.0, 50.0 * 86400.0,
    SearchOptions { step: 3600.0, tolerance: 1e-6, threshold: None },
)?;
```

The event search samples the separation at `step` seconds and refines
extrema with a golden-section search and crossings with bisection. The
step must resolve every feature in the window; the search reports what
it samples rather than guessing about unresolved structure. The
dedicated `search_separation` helper covers extrema and threshold
crossings; the general `search_scalar` engine adds the remaining CSPICE
`RELATE` conditions and backs the seven scalar finders.

```rust
use stkcraft_geocalc::{
    find_distance, Condition, FinderOptions, Interval, SpkKernel,
};

let kernel = SpkKernel::open("de440s.bsp")?;
let windows = [Interval { start: 0.0, end: 30.0 * 86400.0 }];
let near = find_distance(
    &kernel, 301, 399, stkcraft_geocalc::Aberration::None,
    &windows, Condition::GreaterThan, 4.0e5,
    FinderOptions { step: 3600.0, tolerance: 1e-6, adjustment: 0.0 },
)?;
```

Phase angle, angular size, and the IAU body-fixed orientation come from
the same state engine plus a text PCK:

```rust
use stkcraft_geocalc::{angular_size, phase_angle, Aberration, Pck};

let pck = Pck::open("pck00011.tpc")?;
let phase = phase_angle(&kernel, 301, 10, 399, 0.0, Aberration::None)?;
let size = angular_size(&kernel, &pck, 399, 301, 0.0, Aberration::None)?;
let moon_to_body = pck.body_rotation(301, 0.0);
```

Frame rotations mirror CSPICE `pxform`. A frame kernel is loaded once at
startup; `et` is accepted for signature compatibility and is unused by
the time-independent frames supported so far.

```rust
use stkcraft_geocalc::Frames;

let frames = Frames::open("moon_080317.tf")?;
// Rotation taking vectors from MOON_ME_DE421 into MOON_PA_DE421.
let matrix = frames.pxform("MOON_ME_DE421", "MOON_PA_DE421", 0.0)?;
let state_matrix = frames.sxform("J2000", "ECLIPJ2000", 0.0)?;
```

`TKFRAME_MATRIX` is read as column-major, `ANGLES` compose as
`[a1]_axis1 * [a2]_axis2 * [a3]_axis3`, and `TKFRAME_Q` is
`(q0, q1, q2, q3)` with the scalar part first. These conventions were
established against CSPICE, not assumed; see the module documentation in
`src/frames/frame.rs`.

## Strict verification

The acceptance target is the complete computational behavior of NASA NAIF
WebGeocalc. HTTP server compatibility is outside this crate's scope. Time
conversion is owned by the separate STKCraft-time package.

**Full WebGeocalc parity has not been achieved.** Passing a subset of tests,
matching a function name, or falling inside an error tolerance does not close
a missing feature or an unequal output. See [strict repair status](docs/STRICT_PARITY_STATUS.md)
and the [original audit](docs/WGC_CONFORMANCE_REVIEW.md).

The independent numerical reference is CSPICE N0067. The official Windows C
toolkit has been downloaded and its 2,229 C sources extracted under
`target/reference/n0067/cspice/src/cspice`. Source/archive/kernel hashes are
recorded in [reference provenance](docs/reference/CSPICE_N0067.json).
CSPICE is an offline oracle; production calculations remain Rust.

```powershell
python tools/verify_strict.py
```

This command requires the pinned reference kernels, rejects skipped/ignored
checks, runs formatting, Python verifier regressions, and every Rust test,
and writes logs plus a source/fixture hash manifest to
`target/strict-verification/report.json`. The report separately records that
full WebGeocalc parity remains incomplete.

New conformance cases compare actual CSPICE doubles exactly. Geometry Finder
uses the same search step and convergence setting as the reference; those
settings do not become a tolerance for comparing the returned endpoints.
The tests cover source-order SPK interpolation/segment priority, position-only
and state aberration corrections, PCK number parsing/orientation, coordinate
Jacobians, point observers, pointing directions, ellipsoid surface/tangent
geometry, orbital elements, scalar/boolean GF algorithms, windows, occultation
and FOV visibility. Exact-case counts and their limits are documented in the
[strict evidence](docs/review-2026-11-11/surface-orbit-visibility-exact.md).

NASA's API transports rounded presentation values. Those values are preserved
as captured, including unequal results. They are not eligible as an unrounded
CSPICE oracle. The live comparator defaults to strict transported-value
comparison, rejects missing/extra outputs and non-finite numbers, and cannot
turn a diagnostic tolerance pass into conformance:

```powershell
python tools/wgc2_geometry_compare.py --help
```

Reference generators under `tools/gen_*.py` invoke independent CSPICE routines.
Regenerating fixtures must never use this crate's calculated answers.
No hard-coded numerical answer overrides are used.

## License

MIT License for the project's original code. See [LICENSE](LICENSE) and
[ATTRIBUTION.md](ATTRIBUTION.md) for source-adaptation provenance and
upstream notices. This third-party implementation is not NASA-certified.

The SPK kernel used for validation is a NAIF data product and is not
covered by the crate's MIT license. It is never bundled.
