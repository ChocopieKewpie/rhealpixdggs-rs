use crate::error::{Error, Result};

/// WGS84 semi-major axis in metres.
pub const WGS84_A: f64 = 6_378_137.0;
/// WGS84 inverse flattening.
pub const WGS84_INVERSE_FLATTENING: f64 = 298.257_223_563;

/// An oblate ellipsoid of revolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipsoid {
    semi_major_axis: f64,
    flattening: f64,
    eccentricity: f64,
    authalic_radius: f64,
}

impl Ellipsoid {
    /// Build an ellipsoid from its semi-major axis and flattening.
    pub fn new(semi_major_axis: f64, flattening: f64) -> Result<Self> {
        if !semi_major_axis.is_finite() || semi_major_axis <= 0.0 {
            return Err(Error::InvalidCoordinate(
                "semi-major axis must be finite and positive".to_owned(),
            ));
        }
        if !flattening.is_finite() || !(0.0..1.0).contains(&flattening) {
            return Err(Error::InvalidCoordinate(
                "flattening must be finite and in [0, 1)".to_owned(),
            ));
        }
        let eccentricity = (flattening * (2.0 - flattening)).sqrt();
        let authalic_radius = authalic_radius(semi_major_axis, eccentricity);
        Ok(Self {
            semi_major_axis,
            flattening,
            eccentricity,
            authalic_radius,
        })
    }

    /// The WGS84 reference ellipsoid.
    pub fn wgs84() -> Self {
        Self::new(WGS84_A, 1.0 / WGS84_INVERSE_FLATTENING).expect("WGS84 constants are valid")
    }

    /// Construct a sphere.
    pub fn sphere(radius: f64) -> Result<Self> {
        Self::new(radius, 0.0)
    }

    /// Return the semi-major axis in metres.
    pub const fn semi_major_axis(self) -> f64 {
        self.semi_major_axis
    }

    /// Return the flattening.
    pub const fn flattening(self) -> f64 {
        self.flattening
    }

    /// Return the eccentricity.
    pub const fn eccentricity(self) -> f64 {
        self.eccentricity
    }

    /// Return the radius of the equal-area authalic sphere in metres.
    pub const fn authalic_radius(self) -> f64 {
        self.authalic_radius
    }
}

fn authalic_radius(semi_major_axis: f64, eccentricity: f64) -> f64 {
    if eccentricity == 0.0 {
        return semi_major_axis;
    }
    let e = eccentricity;
    let factor = (0.5 * (1.0 - (1.0 - e * e) / (2.0 * e) * ((1.0 - e) / (1.0 + e)).ln())).sqrt();
    semi_major_axis * factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgs84_authalic_radius_matches_reference() {
        let ellipsoid = Ellipsoid::wgs84();
        assert!((ellipsoid.authalic_radius() - 6_371_007.180_918_474).abs() < 1e-6);
        assert!((ellipsoid.eccentricity() - 0.081_819_190_842_621_49).abs() < 1e-15);
    }

    #[test]
    fn sphere_keeps_its_radius() {
        let sphere = Ellipsoid::sphere(2.0).unwrap();
        assert_eq!(sphere.authalic_radius(), 2.0);
        assert_eq!(sphere.eccentricity(), 0.0);
    }
}
