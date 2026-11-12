//! NAIF/SPICE-aligned geometry calculations for STKCraft.
//!
//! This crate reads binary SPK kernels, evaluates geometric and
//! aberration-corrected state vectors, and applies frame-kernel
//! rotations, matching CSPICE.
//!
//! # Scope of this version
//!
//! * DAF container parsing (both byte orders).
//! * All16 N0067 SPK types: 1/2/3/5/8/9/10/12/13/14/15/17/18/19/20/21.
//! * Segment chains, so `state(target, observer, et)` works when the
//!   kernel stores the bodies relative to a common center.
//! * Frame kernels: fixed-rotation `TKFRAME` definitions (`MATRIX`,
//!   `ANGLES`, `QUATERNION`), all 21 built-in inertial frames and text/binary
//!   PCK body frames.
//! * Light-time and stellar-aberration corrections matching CSPICE
//!   `spkezr`: `NONE`, `LT`, `LT+S`, `CN`, `CN+S`, and the
//!   `X`-prefixed transmission variants.
//! * Angular separation matching CSPICE `trgsep`, phase angle matching
//!   `phaseq`, both angular limb extents and typed two-direction separation.
//! * Osculating orbital elements matching CSPICE `oscelt`.
//! * Sub-observer and sub-solar points matching `subpnt` and `subslr`.
//! * Surface intercept points, illumination angles, and planetocentric
//!   solar longitude.
//! * Pointing direction matching CSPICE `spkezp` plus `pxform`.
//! * Apparent states in non-inertial frames matching CSPICE `spkezr`,
//!   including the frame-center epoch evaluation and the light-time-rate
//!   term in the body-fixed velocity.
//! * WebGeocalc state representations (`RECTANGULAR`, `RA_DEC`,
//!   `LATITUDINAL`, `CYLINDRICAL`, `SPHERICAL`, `AZ_EL`,
//!   `PLANETODETIC`, `PLANETOGRAPHIC`) with WGC's column order and
//!   analytic coordinate rates.
//! * Ray-ellipsoid tangent points matching `tangpt`, including the
//!   light-time and stellar-aberration iteration.
//! * Geometry Finder searches for distance, range rate, phase angle,
//!   illumination angles, position coordinates, sub-point coordinates,
//!   and surface-intercept coordinates, with CSPICE `RELATE` conditions.
//! * Coordinate systems: rectangular, latitudinal, RA/Dec, spherical,
//!   cylindrical, geodetic, and planetographic.
//! * Instrument FOV parsing and point/ray visibility (`fovtrg`/`fovray`)
//!   for circle, ellipse, rectangle, and polygon shapes.
//! * Occultation state (`occult`) and searches (`gfoclt`) for triaxial
//!   ellipsoids and point shapes.
//! * Text PCK parsing: body radii, gravitational parameters, and the IAU
//!   body-fixed orientation model (`Pck::body_rotation`) together with its
//!   analytic time derivative (`Pck::body_state_rotation`).
//! * Binary PCK types2/3/20, all six CK types, explicit SCLK context,
//!   default body-frame associations and both byte orders.
//! * Named-frame SPK composition, including rotating source frames, through
//!   explicit Frames/PCK contexts; position and state paths remain distinct.
//!
//! Ellipsoid occultation is classified from the exact apparent-limb
//! conics. Point-target FOV visibility is exact for the four CSPICE FOV
//! shapes. Ellipsoid FOV visibility uses the N0067 analytic predicates.
//! Type-2 DSK has explicit unprioritized surface entry points. Unsupported
//! shape/facade combinations and kernel types return errors.
//!
//! # Alignment
//!
//! Results are checked against CSPICE with committed golden vectors:
//! `spkezr` for geometric and corrected states, and `pxform` for frame
//! rotations. No accuracy is claimed beyond what those vectors
//! demonstrate.

#![forbid(unsafe_code)]

mod ephemeris;
pub mod error;
#[cfg(feature = "mcp")]
pub mod mcp;
pub use ephemeris::spk;
pub(crate) use ephemeris::{sgp4, spk_equinoctial, spk_mda, spkapo, two_body};
mod kernels;
pub(crate) use kernels::{binary_pck, generic_segment};
pub use kernels::{ck, daf, pck};
mod frames;
pub(crate) use frames::{builtin_frames, earth_orientation, wahr_table};
pub use frames::{frame, frame_calculation, orientation};
mod shapes;
pub use shapes::dsk;
pub(crate) use shapes::{dsk_point, dsk_volume, fov_ellipsoid, occult_ellipsoid};
mod geometry;
pub use geometry::{
    coords, direction, fov, location, occult, orbit, pointing, separation, state_rep, surface,
    surface_coordinates, tangent,
};
mod search;
pub use search::{finder, gf_constraints};

pub use coords::{
    coordinate_value, coordinate_value_with_pck, planetographic_east_positive, to_coordinates,
    to_coordinates_with_direction, CoordinateSystem,
};
pub use direction::{
    resolve_ray_directions, resolve_ray_directions_with_pck,
    surface_intercept_geometries_with_model_in, surface_intercept_points,
    surface_intercept_points_in, tangent_points, tangent_points_in, FrameAxis, RayDirection,
};
pub use dsk::{DlaDescriptor, DskDescriptor, DskHit, DskKernel, DskSegment, DskSet};
pub use error::GeoError;
pub use finder::{
    angular_separation, angular_separation_shaped, angular_size, angular_size_extents,
    angular_size_with_method, complement_windows, contract_windows, expand_windows, fill_windows,
    filter_windows, find_angular_separation, find_distance, find_illumination_angle,
    find_illumination_angle_in, find_illumination_angle_with_illuminator, find_phase_angle,
    find_position_coordinate, find_position_coordinate_with_radii, find_range_rate,
    find_sub_point_coordinate, find_sub_point_coordinate_in, find_sub_point_coordinate_with_method,
    find_surface_intercept_coordinate, find_surface_intercept_direction_coordinate,
    find_surface_intercept_direction_coordinate_in, find_surface_intercept_ray_coordinate,
    find_surface_intercept_ray_coordinate_in, find_surface_intercept_ray_coordinate_with_frame,
    normalize_windows, phase_angle, search_scalar, search_scalar_constraint,
    search_scalar_with_decreasing, search_scalar_with_uddc, search_separation,
    search_separation_shaped, AngularShape, AngularSizeMethod, Condition, EventKind, FinderOptions,
    IlluminationAngle, Interval, ScalarConstraint, SearchOptions, SeparationEvent, SubPointKind,
};
pub use fov::{
    ellipsoid_in_fov, ellipsoid_in_fov_with_frame, find_ray_in_fov, find_ray_in_fov_in,
    find_target_in_fov, find_target_in_fov_shaped, find_target_in_fov_shaped_in, ray_in_fov,
    ray_in_fov_in, target_in_fov, target_in_fov_shaped, target_in_fov_shaped_in,
    target_in_fov_with_pck, Fov, FovShape, FovTargetShape, RayAberration,
};
pub use frame::{FrameDef, FrameRotation, Frames, Mat3};
pub use frame_calculation::{frame_transformation, FrameTimeLocation, FrameTransformation};
pub use gf_constraints::{
    apply_scalar_constraint, postprocess_windows, window_durations, DurationUnits,
    IntervalAdjustment, IntervalFiltering, ScalarDomain, WindowDuration, WindowPostprocessing,
};
pub use location::{
    express, point_state, state_between, state_between_in, topocentric_rotation, Endpoint,
    FrameLocus, LocationCoordinates, OutputFrame, PointLocation,
};
pub use occult::{
    find_occultation, find_occultation_with_models_in, occultation_state, OccultShape,
    OccultationModel, OccultationState, OccultationType,
};
pub use orbit::{
    elements_from_state, osculating_elements, osculating_elements_in, GmSource, OsculatingElements,
};
pub use orientation::{
    angular_velocity_vector, describe, describe_with_units, euler_angle_rates,
    euler_angles_with_rates, matrix_to_axis_angle, matrix_to_euler, matrix_to_quaternion,
    AngularUnits, AngularVelocityRepresentation, AngularVelocityUnits, OrientationColumns,
    OrientationRepresentation,
};
pub use pck::Pck;
pub use pointing::{
    pointing_calculation, pointing_direction, pointing_ra_dec, represent_pointing_vector,
    PointingColumns, PointingContext, PointingDirection, PointingMagnitude, PointingRepresentation,
    PointingRequest, PointingResult, PointingVectorCorrection,
};
pub use separation::{angular_separation_directions, DirectionSeparation, SeparationDirection};
pub use spk::{Aberration, CorrectedState, FrameSource, SpkKernel, SpkSegment, State};
pub use state_rep::{
    state_columns, state_columns_with_pck, time_at_target, StateColumns, StateRepresentation,
};
pub use surface::{
    illumination_angles, illumination_geometry, illumination_geometry_in,
    illumination_geometry_with_model_in, nearest_point_on_ellipsoid,
    planetocentric_solar_longitude, planetodetic_to_rectangular, sub_observer_dsk_geometry_in,
    sub_observer_geometry, sub_observer_geometry_in, sub_observer_point,
    sub_observer_point_with_method, sub_solar_dsk_geometry_in, sub_solar_geometry,
    sub_solar_geometry_in, sub_solar_point, sub_solar_point_with_method,
    surface_intercept_geometry, surface_intercept_geometry_in,
    surface_intercept_geometry_with_model_in, surface_intercept_point, surface_intercept_ray,
    DskSubPointMethod, IlluminationAngles, IlluminationGeometry, SubPointMethod, SurfaceGeometry,
    SurfaceModel, SurfacePoint,
};
pub use surface_coordinates::latitudinal_surface_points_in;
pub use tangent::{
    nearest_point_on_ellipsoid_to_line, tangent_point, tangent_point_in, tangent_point_with_locus,
    CorrectionLocus, TangentPoint,
};
