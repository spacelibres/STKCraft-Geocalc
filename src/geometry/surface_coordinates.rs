//! Native surface construction from planetocentric longitude and latitude.
#![allow(clippy::too_many_arguments)]

use crate::{Frames, GeoError, Pck, SpkKernel, SurfaceModel};

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
