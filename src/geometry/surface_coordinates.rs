//! Native surface construction from planetocentric longitude and latitude.
#![allow(clippy::too_many_arguments)]

use crate::{Frames, GeoError, Pck, SpkKernel, SurfaceModel};

/// Error returned by method-string surface coordinate calculations.
#[derive(Debug)]
pub enum SurfaceCoordinateMethodError {
    /// The method string or its surface mapping is invalid.
    Method(crate::SurfaceMethodError),
    /// The geometry calculation or frame validation failed.
    Geometry(GeoError),
}
impl std::fmt::Display for SurfaceCoordinateMethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Method(error) => error.fmt(f),
            Self::Geometry(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for SurfaceCoordinateMethodError {}

/// LATSRF using a NAIF method string and a stateful surface-name resolver.
///
/// This validates the target frame before parsing the method, as LATSRF does.
/// The method's surface list selects the matching loaded DSK segments.
pub fn latitudinal_surface_points_with_method_in(
    ephemeris: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    kernels: &crate::DskSet,
    resolver: &mut crate::SurfaceNameResolver,
    target: i32,
    epoch: f64,
    frame: &str,
    coordinates: &[[f64; 2]],
    method: &str,
) -> Result<Vec<[f64; 3]>, SurfaceCoordinateMethodError> {
    if method.is_empty() || frame.is_empty() {
        return Err(SurfaceCoordinateMethodError::Geometry(GeoError::BadSearch(
            "SPICE(EMPTYSTRING)",
        )));
    }
    let center = frames.center(frame).map_err(|_| {
        SurfaceCoordinateMethodError::Geometry(GeoError::BadSearch("SPICE(NOFRAME)"))
    })?;
    if center != target {
        return Err(SurfaceCoordinateMethodError::Geometry(GeoError::BadSearch(
            "SPICE(INVALIDFRAME)",
        )));
    }
    let parsed = crate::parse_surface_method(resolver, pck, target, method, 100)
        .map_err(SurfaceCoordinateMethodError::Method)?;
    if !parsed.subtype.is_empty() {
        return Err(SurfaceCoordinateMethodError::Geometry(GeoError::BadSearch(
            "SPICE(INVALIDMETHOD)",
        )));
    }
    let model = if parsed.shape == "ELLIPSOID" {
        SurfaceModel::Ellipsoid
    } else {
        SurfaceModel::DskUnprioritized {
            kernels,
            surfaces: &parsed.surfaces,
        }
    };
    latitudinal_surface_points_in(
        ephemeris,
        pck,
        frames,
        target,
        epoch,
        frame,
        coordinates,
        model,
    )
    .map_err(SurfaceCoordinateMethodError::Geometry)
}

/// CSPICE LATSRF with an explicit ellipsoid or unprioritized DSK model.
///
/// Input pairs are `(longitude, latitude)` in radians. Results use the
/// requested target-centered frame. Every requested point must exist;
/// a missing intercept returns the native POINTNOTFOUND error.
pub fn latitudinal_surface_points_in(
    ephemeris: &SpkKernel,
    pck: &Pck,
    frames: &Frames,
    target: i32,
    epoch: f64,
    frame: &str,
    coordinates: &[[f64; 2]],
    model: SurfaceModel<'_>,
) -> Result<Vec<[f64; 3]>, GeoError> {
    let center = frames
        .center(frame)
        .map_err(|_| GeoError::BadSearch("SPICE(NOFRAME)"))?;
    if center != target {
        return Err(GeoError::BadSearch("SPICE(INVALIDFRAME)"));
    }
    match model {
        SurfaceModel::Ellipsoid => {
            let radii = pck
                .radii(target)
                .ok_or(GeoError::BadSearch("SPICE(KERNELVARNOTFOUND)"))?;
            if radii.iter().any(|v| *v <= 0.0) {
                return Err(GeoError::BadSearch("SPICE(BADAXISLENGTH)"));
            }
            coordinates
                .iter()
                .map(|angles| {
                    let point = latrec(1.0, angles[0], angles[1]);
                    // Original EDPNT: do not replace the division by a
                    // reciprocal multiply or normalize this LATREC result.
                    let q = [
                        point[0] / radii[0],
                        point[1] / radii[1],
                        point[2] / radii[2],
                    ];
                    let level = q[0] * q[0] + q[1] * q[1] + q[2] * q[2];
                    if level <= 0.0 {
                        return Err(GeoError::BadSearch("SPICE(POINTTOOSMALL)"));
                    }
                    let scale = level.sqrt();
                    Ok([point[0] / scale, point[1] / scale, point[2] / scale])
                })
                .collect()
        }
        SurfaceModel::DskUnprioritized { kernels, surfaces } => {
            if kernels.kernels().is_empty() {
                return Err(GeoError::BadDas("SPICE(NOLOADEDDSKFILES)"));
            }
            let radius = kernels
                .radial_bounds(ephemeris, pck, frames, target, surfaces)
                .map_err(dsk_error)?
                .1
                * 2.0;
            coordinates
                .iter()
                .map(|angles| {
                    let vertex = latrec(radius, angles[0], angles[1]);
                    let direction = vertex.map(|v| -v);
                    let hit = kernels
                        .ray_intercept_in(
                            ephemeris, pck, frames, target, epoch, frame, vertex, direction,
                            surfaces,
                        )
                        .map_err(dsk_error)?
                        .ok_or(GeoError::BadSearch("SPICE(POINTNOTFOUND)"))?;
                    let point = hit.point;
                    if vertex[0] * point[0] + vertex[1] * point[1] + vertex[2] * point[2] < 0.0 {
                        return Err(GeoError::BadSearch("SPICE(SHAPENOTSUPPORTED)"));
                    }
                    Ok(point)
                })
                .collect()
        }
    }
}

fn latrec(radius: f64, longitude: f64, latitude: f64) -> [f64; 3] {
    [
        radius * longitude.cos() * latitude.cos(),
        radius * longitude.sin() * latitude.cos(),
        radius * latitude.sin(),
    ]
}

fn dsk_error(error: GeoError) -> GeoError {
    match error {
        GeoError::BadDas(
            "no DSK segments for target"
            | "no DSK data match requested surfaces"
            | "no DSK data match target and surfaces",
        ) => GeoError::BadSearch("SPICE(DSKDATANOTFOUND)"),
        other => other,
    }
}
