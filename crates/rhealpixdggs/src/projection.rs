//! Dependency-free HEALPix and rHEALPix projection mathematics.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use crate::ellipsoid::Ellipsoid;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Region {
    NorthPolar,
    Equatorial,
    SouthPolar,
}

/// Project longitude/latitude degrees to the rHEALPix plane in metres.
pub(crate) fn forward(
    ellipsoid: Ellipsoid,
    longitude: f64,
    latitude: f64,
    north_square: u8,
    south_square: u8,
) -> Result<(f64, f64)> {
    validate_lonlat(longitude, latitude)?;
    let lambda = wrap_longitude(longitude.to_radians());
    let phi = latitude.to_radians();
    let beta = authalic_latitude(phi, ellipsoid.eccentricity());
    let (x, y) = healpix_sphere(lambda, beta);
    let (x, y) = combine_triangles(x, y, north_square, south_square, false, None);
    let radius = ellipsoid.authalic_radius();
    Ok((x * radius, y * radius))
}

/// Invert rHEALPix metres to longitude/latitude degrees.
pub(crate) fn inverse(
    ellipsoid: Ellipsoid,
    x: f64,
    y: f64,
    north_square: u8,
    south_square: u8,
) -> Result<(f64, f64)> {
    inverse_with_region(ellipsoid, x, y, north_square, south_square, None)
}

/// Invert rHEALPix metres while resolving face-boundary rounding with an
/// explicit projection region.
pub(crate) fn inverse_in_region(
    ellipsoid: Ellipsoid,
    x: f64,
    y: f64,
    north_square: u8,
    south_square: u8,
    region: Region,
) -> Result<(f64, f64)> {
    inverse_with_region(ellipsoid, x, y, north_square, south_square, Some(region))
}

fn inverse_with_region(
    ellipsoid: Ellipsoid,
    x: f64,
    y: f64,
    north_square: u8,
    south_square: u8,
    region: Option<Region>,
) -> Result<(f64, f64)> {
    if !x.is_finite() || !y.is_finite() {
        return Err(Error::InvalidCoordinate(
            "projected coordinates must be finite".to_owned(),
        ));
    }
    let radius = ellipsoid.authalic_radius();
    let (x, y) = (x / radius, y / radius);
    if !in_rhealpix_image(x, y, north_square, south_square) {
        return Err(Error::OutsideProjection);
    }
    let (x, y) = combine_triangles(x, y, north_square, south_square, true, region);
    let (lambda, beta) = healpix_sphere_inverse(x, y)?;
    let phi = common_latitude(beta, ellipsoid.eccentricity());
    Ok((wrap_longitude(lambda).to_degrees(), phi.to_degrees()))
}

fn validate_lonlat(longitude: f64, latitude: f64) -> Result<()> {
    if !longitude.is_finite() || !latitude.is_finite() {
        return Err(Error::InvalidCoordinate(
            "longitude and latitude must be finite".to_owned(),
        ));
    }
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(Error::InvalidCoordinate(format!(
            "latitude {latitude} is outside [-90, 90]"
        )));
    }
    Ok(())
}

fn wrap_longitude(longitude: f64) -> f64 {
    (longitude + PI).rem_euclid(2.0 * PI) - PI
}

fn authalic_latitude(phi: f64, eccentricity: f64) -> f64 {
    if eccentricity == 0.0 {
        return phi;
    }
    let e = eccentricity;
    let root = (1.0 - e * e).sqrt();
    let n = (1.0 - root) / (1.0 + root);

    phi + n
        * (-4.0 / 3.0
            + n * (-4.0 / 45.0
                + n * (88.0 / 315.0
                    + n * (538.0 / 4725.0
                        + n * (20_824.0 / 467_775.0 + n * (-44_732.0 / 2_837_835.0))))))
        * (2.0 * phi).sin()
        + n.powi(2)
            * (34.0 / 45.0
                + n * (8.0 / 105.0
                    + n * (-2482.0 / 14_175.0
                        + n * (-37_192.0 / 467_775.0 + n * (-12_467_764.0 / 212_837_625.0)))))
            * (4.0 * phi).sin()
        + n.powi(3)
            * (-1532.0 / 2835.0
                + n * (-898.0 / 14_175.0
                    + n * (54_968.0 / 467_775.0 + n * 100_320_856.0 / 1_915_538_625.0)))
            * (6.0 * phi).sin()
        + n.powi(4)
            * (6007.0 / 14_175.0 + n * (24_496.0 / 467_775.0 + n * (-5_884_124.0 / 70_945_875.0)))
            * (8.0 * phi).sin()
        + n.powi(5) * (-23_356.0 / 66_825.0 + n * (-839_792.0 / 19_348_875.0)) * (10.0 * phi).sin()
        + n.powi(6) * (570_284_222.0 / 1_915_538_625.0) * (12.0 * phi).sin()
}

fn common_latitude(beta: f64, eccentricity: f64) -> f64 {
    if eccentricity == 0.0 {
        return beta;
    }
    let e = eccentricity;
    let root = (1.0 - e * e).sqrt();
    let n = (1.0 - root) / (1.0 + root);

    beta + n
        * (4.0 / 3.0
            + n * (4.0 / 45.0
                + n * (-16.0 / 35.0
                    + n * (-2582.0 / 14_175.0
                        + n * (60_136.0 / 467_775.0 + n * 28_112_932.0 / 212_837_625.0)))))
        * (2.0 * beta).sin()
        + n.powi(2)
            * (46.0 / 45.0
                + n * (152.0 / 945.0
                    + n * (-11_966.0 / 14_175.0
                        + n * (-21_016.0 / 51_975.0 + n * 251_310_128.0 / 638_512_875.0))))
            * (4.0 * beta).sin()
        + n.powi(3)
            * (3044.0 / 2835.0
                + n * (3802.0 / 14_175.0
                    + n * (-94_388.0 / 66_825.0 + n * (-8_797_648.0 / 10_945_935.0))))
            * (6.0 * beta).sin()
        + n.powi(4)
            * (6059.0 / 4725.0 + n * (41_072.0 / 93_555.0 + n * (-1_472_637_812.0 / 638_512_875.0)))
            * (8.0 * beta).sin()
        + n.powi(5)
            * (768_272.0 / 467_775.0 + n * 455_935_736.0 / 638_512_875.0)
            * (10.0 * beta).sin()
        + n.powi(6) * (4_210_684_958.0 / 1_915_538_625.0) * (12.0 * beta).sin()
}

fn healpix_sphere(lambda: f64, phi: f64) -> (f64, f64) {
    let phi_zero = (2.0_f64 / 3.0).asin();
    if phi.abs() <= phi_zero {
        (lambda, 3.0 * PI / 8.0 * phi.sin())
    } else {
        let sigma = (3.0 * (1.0 - phi.sin().abs())).sqrt();
        let cap = ((2.0 * lambda / PI + 2.0).floor() as i32).min(3);
        let lambda_center = -3.0 * PI / 4.0 + FRAC_PI_2 * f64::from(cap);
        (
            lambda_center + (lambda - lambda_center) * sigma,
            phi.signum() * FRAC_PI_4 * (2.0 - sigma),
        )
    }
}

fn healpix_sphere_inverse(x: f64, y: f64) -> Result<(f64, f64)> {
    if y.abs() <= FRAC_PI_4 {
        return Ok((x, (8.0 * y / (3.0 * PI)).asin()));
    }
    if y.abs() < FRAC_PI_2 {
        let cap = ((2.0 * x / PI + 2.0).floor() as i32).min(3);
        let x_center = -3.0 * PI / 4.0 + FRAC_PI_2 * f64::from(cap);
        let tau = 2.0 - 4.0 * y.abs() / PI;
        let lambda = (x_center + (x - x_center) / tau).clamp(-PI, PI);
        let phi = y.signum() * (1.0 - tau * tau / 3.0).asin();
        return Ok((lambda, phi));
    }
    if (y.abs() - FRAC_PI_2).abs() <= 1e-12 {
        return Ok((-PI, y.signum() * FRAC_PI_2));
    }
    Err(Error::OutsideProjection)
}

fn region(y: f64) -> Region {
    if y > FRAC_PI_4 {
        Region::NorthPolar
    } else if y < -FRAC_PI_4 {
        Region::SouthPolar
    } else {
        Region::Equatorial
    }
}

pub(crate) fn triangle_number(
    x: f64,
    y: f64,
    north_square: u8,
    south_square: u8,
    inverse: bool,
) -> (i32, Region) {
    triangle_number_with_region(x, y, north_square, south_square, inverse, None)
}

fn triangle_number_with_region(
    x: f64,
    y: f64,
    north_square: u8,
    south_square: u8,
    inverse: bool,
    region_hint: Option<Region>,
) -> (i32, Region) {
    let region = region_hint.unwrap_or_else(|| region(y));
    if region == Region::Equatorial {
        return (0, region);
    }
    if !inverse {
        let number = if x < -FRAC_PI_2 {
            0
        } else if x < 0.0 {
            1
        } else if x < FRAC_PI_2 {
            2
        } else {
            3
        };
        return (number, region);
    }

    let epsilon = 1e-15;
    let square = match region {
        Region::NorthPolar => i32::from(north_square),
        Region::SouthPolar => i32::from(south_square),
        Region::Equatorial => unreachable!(),
    };
    let number = match region {
        Region::NorthPolar => {
            let line_1 = x - (-3.0 * PI / 4.0 + f64::from(square - 1) * FRAC_PI_2);
            let line_2 = -x + (-3.0 * PI / 4.0 + f64::from(square + 1) * FRAC_PI_2);
            if y < line_1 - epsilon && y >= line_2 - epsilon {
                (square + 1).rem_euclid(4)
            } else if y >= line_1 - epsilon && y > line_2 + epsilon {
                (square + 2).rem_euclid(4)
            } else if y > line_1 + epsilon && y <= line_2 + epsilon {
                (square + 3).rem_euclid(4)
            } else {
                square
            }
        }
        Region::SouthPolar => {
            let line_1 = x - (-3.0 * PI / 4.0 + f64::from(square + 1) * FRAC_PI_2);
            let line_2 = -x + (-3.0 * PI / 4.0 + f64::from(square - 1) * FRAC_PI_2);
            if y <= line_1 + epsilon && y > line_2 + epsilon {
                (square + 1).rem_euclid(4)
            } else if y < line_1 - epsilon && y <= line_2 + epsilon {
                (square + 2).rem_euclid(4)
            } else if y >= line_1 - epsilon && y < line_2 - epsilon {
                (square + 3).rem_euclid(4)
            } else {
                square
            }
        }
        Region::Equatorial => unreachable!(),
    };
    (number, region)
}

fn combine_triangles(
    x: f64,
    y: f64,
    north_square: u8,
    south_square: u8,
    inverse: bool,
    region_hint: Option<Region>,
) -> (f64, f64) {
    let north_square = north_square % 4;
    let south_square = south_square % 4;
    let (triangle, region) =
        triangle_number_with_region(x, y, north_square, south_square, inverse, region_hint);
    if region == Region::Equatorial {
        return (x, y);
    }
    let triangle_center = (
        -3.0 * PI / 4.0 + f64::from(triangle) * FRAC_PI_2,
        y.signum() * FRAC_PI_2,
    );
    let square = match region {
        Region::NorthPolar => i32::from(north_square),
        Region::SouthPolar => i32::from(south_square),
        Region::Equatorial => unreachable!(),
    };
    let tip = (
        -3.0 * PI / 4.0 + f64::from(square) * FRAC_PI_2,
        match region {
            Region::NorthPolar => FRAC_PI_2,
            Region::SouthPolar => -FRAC_PI_2,
            Region::Equatorial => unreachable!(),
        },
    );

    if !inverse {
        let turns = match region {
            Region::NorthPolar => triangle - square,
            Region::SouthPolar => -(triangle - square),
            Region::Equatorial => unreachable!(),
        };
        let rotated = rotate_quarter((x - triangle_center.0, y - triangle_center.1), turns);
        (rotated.0 + tip.0, rotated.1 + tip.1)
    } else {
        let turns = match region {
            Region::NorthPolar => -(triangle - square),
            Region::SouthPolar => triangle - square,
            Region::Equatorial => unreachable!(),
        };
        let rotated = rotate_quarter((x - tip.0, y - tip.1), turns);
        (rotated.0 + triangle_center.0, rotated.1 + triangle_center.1)
    }
}

fn rotate_quarter(point: (f64, f64), turns: i32) -> (f64, f64) {
    match turns.rem_euclid(4) {
        0 => point,
        1 => (-point.1, point.0),
        2 => (-point.0, -point.1),
        3 => (point.1, -point.0),
        _ => unreachable!(),
    }
}

fn in_rhealpix_image(x: f64, y: f64, north_square: u8, south_square: u8) -> bool {
    let epsilon = 1e-12;
    if (-PI - epsilon..=PI + epsilon).contains(&x)
        && (-FRAC_PI_4 - epsilon..=FRAC_PI_4 + epsilon).contains(&y)
    {
        return true;
    }
    let north_left = -PI + f64::from(north_square % 4) * FRAC_PI_2;
    let south_left = -PI + f64::from(south_square % 4) * FRAC_PI_2;
    (north_left - epsilon..=north_left + FRAC_PI_2 + epsilon).contains(&x)
        && (FRAC_PI_4 - epsilon..=3.0 * FRAC_PI_4 + epsilon).contains(&y)
        || (south_left - epsilon..=south_left + FRAC_PI_2 + epsilon).contains(&x)
            && (-3.0 * FRAC_PI_4 - epsilon..=-FRAC_PI_4 + epsilon).contains(&y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: (f64, f64), right: (f64, f64), tolerance: f64) {
        assert!(
            (left.0 - right.0).abs() < tolerance,
            "{left:?} != {right:?}"
        );
        assert!(
            (left.1 - right.1).abs() < tolerance,
            "{left:?} != {right:?}"
        );
    }

    #[test]
    fn sphere_projection_matches_upstream_doctest() {
        let sphere = Ellipsoid::sphere(2.0).unwrap();
        let point = forward(sphere, 0.0, 60.0, 1, 2).unwrap();
        assert_close(
            point,
            (-0.574_951_359_778_215, 2.145_747_686_573_111),
            1e-14,
        );
    }

    #[test]
    fn projection_round_trips_world_samples() {
        let ellipsoid = Ellipsoid::wgs84();
        for point in [
            (0.0, 0.0),
            (174.763_3, -36.848_5),
            (175.611, -40.356),
            (-179.999, 72.5),
            (45.0, 89.0),
            (120.0, -89.0),
        ] {
            let projected = forward(ellipsoid, point.0, point.1, 0, 0).unwrap();
            let result = inverse(ellipsoid, projected.0, projected.1, 0, 0).unwrap();
            assert_close(result, point, 2e-10);
        }
    }
}
