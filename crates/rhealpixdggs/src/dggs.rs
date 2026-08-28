use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use crate::cell::{CellId, Face, validate_resolution};
use crate::ellipsoid::Ellipsoid;
use crate::error::{Error, Result};
use crate::projection;

const N_SIDE: u64 = 3;

/// An aperture-9 rHEALPix discrete global grid system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RhealpixDggs {
    ellipsoid: Ellipsoid,
    north_square: u8,
    south_square: u8,
}

impl RhealpixDggs {
    /// Construct an aperture-9 DGGS with configurable polar-square placement.
    pub const fn new(ellipsoid: Ellipsoid, north_square: u8, south_square: u8) -> Self {
        Self {
            ellipsoid,
            north_square: north_square % 4,
            south_square: south_square % 4,
        }
    }

    /// The upstream-compatible `WGS84_003` configuration.
    pub fn wgs84_003() -> Self {
        Self::new(Ellipsoid::wgs84(), 0, 0)
    }

    /// Return the underlying ellipsoid.
    pub const fn ellipsoid(self) -> Ellipsoid {
        self.ellipsoid
    }

    /// Return the north polar-square position in `0..=3`.
    pub const fn north_square(self) -> u8 {
        self.north_square
    }

    /// Return the south polar-square position in `0..=3`.
    pub const fn south_square(self) -> u8 {
        self.south_square
    }

    /// Convert longitude/latitude degrees to a cell identifier.
    pub fn cell_from_lonlat(
        &self,
        longitude: f64,
        latitude: f64,
        resolution: u8,
    ) -> Result<CellId> {
        validate_resolution(resolution)?;
        let (x, y) = projection::forward(
            self.ellipsoid,
            longitude,
            latitude,
            self.north_square,
            self.south_square,
        )?;
        self.cell_from_projected(x, y, resolution)
    }

    /// Convert rHEALPix planar metres to a cell identifier.
    pub fn cell_from_projected(&self, x: f64, y: f64, resolution: u8) -> Result<CellId> {
        validate_resolution(resolution)?;
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::InvalidCoordinate(
                "projected coordinates must be finite".to_owned(),
            ));
        }
        let radius = self.ellipsoid.authalic_radius();
        let ns = f64::from(self.north_square);
        let ss = f64::from(self.south_square);

        let face = if y > radius * FRAC_PI_4
            && y < radius * 3.0 * FRAC_PI_4
            && x > radius * (-PI + ns * FRAC_PI_2)
            && x < radius * (-FRAC_PI_2 + ns * FRAC_PI_2)
        {
            Face::N
        } else if y > -radius * 3.0 * FRAC_PI_4
            && y < -radius * FRAC_PI_4
            && x > radius * (-PI + ss * FRAC_PI_2)
            && x < radius * (-FRAC_PI_2 + ss * FRAC_PI_2)
        {
            Face::S
        } else if (-radius * FRAC_PI_4..=radius * FRAC_PI_4).contains(&y)
            && (-radius * PI..-radius * FRAC_PI_2).contains(&x)
        {
            Face::O
        } else if (-radius * FRAC_PI_4..=radius * FRAC_PI_4).contains(&y)
            && (-radius * FRAC_PI_2..0.0).contains(&x)
        {
            Face::P
        } else if (-radius * FRAC_PI_4..=radius * FRAC_PI_4).contains(&y)
            && (0.0..radius * FRAC_PI_2).contains(&x)
        {
            Face::Q
        } else if (-radius * FRAC_PI_4..=radius * FRAC_PI_4).contains(&y)
            && (radius * FRAC_PI_2..radius * PI).contains(&x)
        {
            Face::R
        } else {
            return Err(Error::OutsideProjection);
        };

        if resolution == 0 {
            return CellId::new(face, Vec::new());
        }

        let (upper_left_x, upper_left_y) = self.root_upper_left(face);
        let root_width = self.cell_width(0)?;
        let dx = ((x - upper_left_x).abs() / root_width).clamp(0.0, 1.0);
        let dy = ((y - upper_left_y).abs() / root_width).clamp(0.0, 1.0);
        let scale = N_SIDE.pow(u32::from(resolution));
        let column = ((dx * scale as f64).floor() as u64).min(scale - 1);
        let row = ((dy * scale as f64).floor() as u64).min(scale - 1);

        let mut digits = Vec::with_capacity(usize::from(resolution));
        for position in 0..resolution {
            let divisor = N_SIDE.pow(u32::from(resolution - position - 1));
            let child_row = (row / divisor) % N_SIDE;
            let child_column = (column / divisor) % N_SIDE;
            digits.push((child_row * N_SIDE + child_column) as u8);
        }
        CellId::new(face, digits)
    }

    /// Return the projected nucleus (cell-square centre) in metres.
    pub fn cell_to_projected(&self, cell: &CellId) -> Result<(f64, f64)> {
        let (x, y) = self.cell_upper_left(cell)?;
        let half_width = self.cell_width(cell.resolution())? / 2.0;
        Ok((x + half_width, y - half_width))
    }

    /// Return the cell nucleus as longitude/latitude degrees.
    ///
    /// This corresponds to `Cell.nucleus(plane=False)` in `rhealpixdggs-py`.
    pub fn cell_to_lonlat(&self, cell: &CellId) -> Result<(f64, f64)> {
        let (x, y) = self.cell_to_projected(cell)?;
        projection::inverse(self.ellipsoid, x, y, self.north_square, self.south_square)
    }

    /// Return the four projected square vertices, clockwise from upper-left.
    pub fn cell_vertices_projected(&self, cell: &CellId) -> Result<[(f64, f64); 4]> {
        let upper_left = self.cell_upper_left(cell)?;
        let width = self.cell_width(cell.resolution())?;
        Ok([
            upper_left,
            (upper_left.0 + width, upper_left.1),
            (upper_left.0 + width, upper_left.1 - width),
            (upper_left.0, upper_left.1 - width),
        ])
    }

    /// Return four inverse-projected boundary points as longitude/latitude.
    ///
    /// Polar dart cells contain one repeated/non-vertex geographic point, as
    /// in the upstream four-point representation. Shape-aware trimming is a
    /// planned compatibility milestone.
    pub fn cell_vertices_lonlat(&self, cell: &CellId) -> Result<[(f64, f64); 4]> {
        let vertices = self.cell_vertices_projected(cell)?;
        let mut result = [(0.0, 0.0); 4];
        for (index, point) in vertices.into_iter().enumerate() {
            result[index] = projection::inverse(
                self.ellipsoid,
                point.0,
                point.1,
                self.north_square,
                self.south_square,
            )?;
        }
        Ok(result)
    }

    /// Return planar cell width in metres.
    pub fn cell_width(&self, resolution: u8) -> Result<f64> {
        validate_resolution(resolution)?;
        Ok(self.ellipsoid.authalic_radius() * FRAC_PI_2 * 3.0_f64.powi(-i32::from(resolution)))
    }

    /// Return equal-area ellipsoidal cell area in square metres.
    pub fn cell_area(&self, resolution: u8) -> Result<f64> {
        let width = self.cell_width(resolution)?;
        Ok(8.0 / (3.0 * PI) * width * width)
    }

    fn root_upper_left(&self, face: Face) -> (f64, f64) {
        let unit = match face {
            Face::N => (
                -PI + f64::from(self.north_square) * FRAC_PI_2,
                3.0 * FRAC_PI_4,
            ),
            Face::O => (-PI, FRAC_PI_4),
            Face::P => (-FRAC_PI_2, FRAC_PI_4),
            Face::Q => (0.0, FRAC_PI_4),
            Face::R => (FRAC_PI_2, FRAC_PI_4),
            Face::S => (-PI + f64::from(self.south_square) * FRAC_PI_2, -FRAC_PI_4),
        };
        let radius = self.ellipsoid.authalic_radius();
        (unit.0 * radius, unit.1 * radius)
    }

    fn cell_upper_left(&self, cell: &CellId) -> Result<(f64, f64)> {
        validate_resolution(cell.resolution())?;
        let (root_x, root_y) = self.root_upper_left(cell.face());
        let mut row = 0_u64;
        let mut column = 0_u64;
        for digit in cell.digits() {
            row = row * N_SIDE + u64::from(*digit) / N_SIDE;
            column = column * N_SIDE + u64::from(*digit) % N_SIDE;
        }
        let scale = N_SIDE.pow(u32::from(cell.resolution()));
        let root_width = self.cell_width(0)?;
        Ok((
            root_x + root_width * column as f64 / scale as f64,
            root_y - root_width * row as f64 / scale as f64,
        ))
    }
}

impl Default for RhealpixDggs {
    fn default() -> Self {
        Self::wgs84_003()
    }
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
    fn origin_matches_upstream_cell_example() {
        let dggs = RhealpixDggs::wgs84_003();
        assert_eq!(
            dggs.cell_from_projected(0.0, 0.0, 1).unwrap().to_string(),
            "Q3"
        );
    }

    #[test]
    fn planar_nucleus_matches_upstream_reference() {
        let dggs = RhealpixDggs::wgs84_003();
        let cell: CellId = "P57".parse().unwrap();
        assert_close(
            dggs.cell_to_projected(&cell).unwrap(),
            (-1_667_925.779_628_441_7, -1_111_950.519_752_294),
            1e-6,
        );
    }

    #[test]
    fn area_is_equal_and_scales_by_nine() {
        let dggs = RhealpixDggs::wgs84_003();
        let area_zero = dggs.cell_area(0).unwrap();
        let area_one = dggs.cell_area(1).unwrap();
        assert!((area_zero / area_one - 9.0).abs() < 1e-12);
        assert!((area_zero - 85_010_936_954_014.78).abs() < 0.1);
    }

    #[test]
    fn point_cell_nucleus_stays_in_same_cell() {
        let dggs = RhealpixDggs::wgs84_003();
        for point in [
            (174.763_3, -36.848_5),
            (175.611, -40.356),
            (-122.419_4, 37.774_9),
            (0.0, 80.0),
            (120.0, -80.0),
        ] {
            let cell = dggs.cell_from_lonlat(point.0, point.1, 8).unwrap();
            let nucleus = dggs.cell_to_lonlat(&cell).unwrap();
            assert_eq!(
                dggs.cell_from_lonlat(nucleus.0, nucleus.1, 8).unwrap(),
                cell
            );
        }
    }

    #[test]
    fn wgs84_cells_match_upstream_golden_cases() {
        let dggs = RhealpixDggs::wgs84_003();
        let cases = [
            (0.0, 0.0, 1, "Q3"),
            (174.7633, -36.8485, 8, "R88446545"),
            (175.611, -40.356, 12, "R887560473610"),
            (-122.4194, 37.7749, 15, "O125051437212863"),
            (-179.999, 72.5, 12, "N622446670001"),
            (45.0, 89.0, 12, "N444147711147"),
            (120.0, -89.0, 15, "S444375206675068"),
        ];
        for (longitude, latitude, resolution, expected) in cases {
            assert_eq!(
                dggs.cell_from_lonlat(longitude, latitude, resolution)
                    .unwrap()
                    .to_string(),
                expected
            );
        }
    }
}
