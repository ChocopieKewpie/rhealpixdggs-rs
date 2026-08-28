use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use crate::cell::{
    CellId, CellShape, Direction, EllipsoidalDirection, Face, Region, validate_resolution,
};
use crate::ellipsoid::Ellipsoid;
use crate::error::{Error, Result};
use crate::projection;

const N_SIDE: u64 = 3;
const MAX_BOUNDARY_POINTS: usize = 10_000_000;

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

    /// Project longitude/latitude degrees to rHEALPix metres.
    pub fn project_lonlat(&self, longitude: f64, latitude: f64) -> Result<(f64, f64)> {
        projection::forward(
            self.ellipsoid,
            longitude,
            latitude,
            self.north_square,
            self.south_square,
        )
    }

    /// Invert rHEALPix metres to longitude/latitude degrees.
    pub fn unproject_lonlat(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        projection::inverse(self.ellipsoid, x, y, self.north_square, self.south_square)
    }

    /// Invert rHEALPix metres with an explicit geographic region hint.
    pub fn unproject_lonlat_in_region(&self, x: f64, y: f64, region: Region) -> Result<(f64, f64)> {
        projection::inverse_in_region(
            self.ellipsoid,
            x,
            y,
            self.north_square,
            self.south_square,
            projection_region(region),
        )
    }

    /// Project longitude/latitude degrees to HEALPix metres.
    pub fn project_healpix_lonlat(&self, longitude: f64, latitude: f64) -> Result<(f64, f64)> {
        projection::healpix_forward(self.ellipsoid, longitude, latitude)
    }

    /// Invert HEALPix metres to longitude/latitude degrees.
    pub fn unproject_healpix_lonlat(&self, x: f64, y: f64) -> Result<(f64, f64)> {
        projection::healpix_inverse(self.ellipsoid, x, y)
    }

    /// Transform between HEALPix and rHEALPix projected metres.
    pub fn combine_triangles(
        &self,
        x: f64,
        y: f64,
        inverse: bool,
        region: Option<Region>,
    ) -> Result<(f64, f64)> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::InvalidCoordinate(
                "projected coordinates must be finite".to_owned(),
            ));
        }
        let radius = self.ellipsoid.authalic_radius();
        let transformed = projection::combine_triangles(
            x / radius,
            y / radius,
            self.north_square,
            self.south_square,
            inverse,
            region.map(projection_region),
        );
        Ok((transformed.0 * radius, transformed.1 * radius))
    }

    /// Identify the HEALPix triangle and geographic region of projected metres.
    pub fn triangle(&self, x: f64, y: f64, inverse: bool) -> Result<(Option<u8>, Region)> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::InvalidCoordinate(
                "projected coordinates must be finite".to_owned(),
            ));
        }
        let radius = self.ellipsoid.authalic_radius();
        let (number, region) = projection::triangle_number(
            x / radius,
            y / radius,
            self.north_square,
            self.south_square,
            inverse,
        );
        let region = cell_region(region);
        Ok((
            (region != Region::Equatorial).then_some(number as u8),
            region,
        ))
    }

    /// Return geocentric Cartesian coordinates for longitude/latitude degrees.
    pub fn xyz_lonlat(&self, longitude: f64, latitude: f64) -> Result<(f64, f64, f64)> {
        self.project_lonlat(longitude, latitude)?;
        let longitude = longitude.to_radians();
        let latitude = latitude.to_radians();
        let eccentricity = self.ellipsoid.eccentricity();
        let normal = self.ellipsoid.semi_major_axis()
            / (1.0 - eccentricity.powi(2) * latitude.sin().powi(2)).sqrt();
        Ok((
            normal * longitude.cos() * latitude.cos(),
            normal * longitude.sin() * latitude.cos(),
            normal * (1.0 - eccentricity.powi(2)) * latitude.sin(),
        ))
    }

    /// Return geocentric Cartesian coordinates for an rHEALPix projected point.
    pub fn xyz_projected(&self, x: f64, y: f64) -> Result<(f64, f64, f64)> {
        let (longitude, latitude) = self.unproject_lonlat(x, y)?;
        self.xyz_lonlat(longitude, latitude)
    }

    /// Fold an rHEALPix projected point onto a cube centred at the origin.
    pub fn xyz_cube_projected(&self, x: f64, y: f64) -> Result<(f64, f64, f64)> {
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::InvalidCoordinate(
                "projected coordinates must be finite".to_owned(),
            ));
        }
        let width = self.cell_width(0)?;
        let mut x = x + 2.0 * width;
        let y = y + width / 2.0;
        let point = if y < 0.0 {
            x -= f64::from(self.south_square) * width;
            match self.south_square {
                0 => (x, 0.0, y),
                1 => (y + width, 0.0, -x),
                2 => (width - x, 0.0, -y - width),
                3 => (-y, 0.0, x - width),
                _ => unreachable!(),
            }
        } else if y > width {
            x -= f64::from(self.north_square) * width;
            match self.north_square {
                0 => (x, width, -y + width),
                1 => (-y + 2.0 * width, width, -x),
                2 => (-x + width, width, y - 2.0 * width),
                3 => (y - width, width, x - width),
                _ => unreachable!(),
            }
        } else if x < width {
            (x, y, 0.0)
        } else if x < 2.0 * width {
            x -= width;
            (width, y, -x)
        } else if x < 3.0 * width {
            x -= 2.0 * width;
            (width - x, y, -width)
        } else {
            x -= 3.0 * width;
            (0.0, y, x - width)
        };
        Ok((
            point.0 - width / 2.0,
            point.1 - width / 2.0,
            point.2 + width / 2.0,
        ))
    }

    /// Project longitude/latitude degrees and fold the result onto a cube.
    pub fn xyz_cube_lonlat(&self, longitude: f64, latitude: f64) -> Result<(f64, f64, f64)> {
        let (x, y) = self.project_lonlat(longitude, latitude)?;
        self.xyz_cube_projected(x, y)
    }

    /// Convert longitude/latitude degrees to a cell identifier.
    pub fn cell_from_lonlat(
        &self,
        longitude: f64,
        latitude: f64,
        resolution: u8,
    ) -> Result<CellId> {
        validate_resolution(resolution)?;
        let (x, y) = self.project_lonlat(longitude, latitude)?;
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
        let column = stable_grid_index(dx, scale);
        let row = stable_grid_index(dy, scale);

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

    /// Return the planar upper-left vertex used by the hierarchical square.
    pub fn cell_upper_left_projected(&self, cell: &CellId) -> Result<(f64, f64)> {
        self.cell_upper_left(cell)
    }

    /// Return the ellipsoidal projection of the planar upper-left vertex.
    pub fn cell_upper_left_lonlat(&self, cell: &CellId) -> Result<(f64, f64)> {
        let point = self.cell_upper_left(cell)?;
        self.unproject_lonlat_in_region(point.0, point.1, cell.region())
    }

    /// Return the projected location of the ellipsoidal northwest vertex.
    pub fn cell_northwest_vertex_projected(&self, cell: &CellId) -> Result<(f64, f64)> {
        let vertices = self.cell_vertices_projected(cell)?;
        Ok(vertices[self.northwest_vertex_index(cell, &vertices)?])
    }

    /// Return the ellipsoidal northwest vertex.
    pub fn cell_northwest_vertex_lonlat(&self, cell: &CellId) -> Result<(f64, f64)> {
        let point = self.cell_northwest_vertex_projected(cell)?;
        self.unproject_lonlat_in_region(point.0, point.1, cell.region())
    }

    /// Return inverse-projected boundary points as longitude/latitude.
    ///
    /// Points begin at the geographic northwest vertex and proceed clockwise,
    /// matching `Cell.vertices(plane=False)` in `rhealpixdggs-py`. When
    /// `trim_dart` is true, the non-vertex point of a triangular dart is
    /// removed.
    pub fn cell_vertices_lonlat(&self, cell: &CellId, trim_dart: bool) -> Result<Vec<(f64, f64)>> {
        let planar = self.cell_vertices_projected(cell)?;
        let northwest = self.northwest_vertex_index(cell, &planar)?;
        let mut result = Vec::with_capacity(4);
        for offset in 0..4 {
            let point = planar[(northwest + offset) % 4];
            result.push(projection::inverse_in_region(
                self.ellipsoid,
                point.0,
                point.1,
                self.north_square,
                self.south_square,
                projection_region(cell.region()),
            )?);
        }
        if trim_dart && cell.shape() == CellShape::Dart {
            let non_vertex = match cell.region() {
                Region::NorthPolar => 2,
                Region::SouthPolar => 1,
                Region::Equatorial => unreachable!("dart cells are polar"),
            };
            result.remove(non_vertex);
        }
        Ok(result)
    }

    /// Return a clockwise, densified square boundary in projected metres.
    ///
    /// `points_per_edge` includes both corners of each edge and must be at
    /// least two. Shared corners occur once, so the result always contains
    /// exactly `4 * points_per_edge - 4` points. Points start at the planar
    /// upper-left corner. When `interior` is true, the boundary is inset by
    /// one ten-thousandth of the cell width, matching `rhealpixdggs-py`.
    pub fn cell_boundary_projected(
        &self,
        cell: &CellId,
        points_per_edge: usize,
        interior: bool,
    ) -> Result<Vec<(f64, f64)>> {
        let point_count = boundary_point_count(points_per_edge)?;
        let upper_left = self.cell_upper_left(cell)?;
        let width = self.cell_width(cell.resolution())?;
        let inset = if interior { width / 10_000.0 } else { 0.0 };
        let step = (width - 2.0 * inset) / (points_per_edge - 1) as f64;
        let mut edge_start = (upper_left.0 + inset, upper_left.1 - inset);
        let mut result = Vec::with_capacity(point_count + 1);
        result.push(edge_start);

        for (dx, dy) in [(1.0, 0.0), (0.0, -1.0), (-1.0, 0.0), (0.0, 1.0)] {
            for offset in 1..points_per_edge {
                let offset = offset as f64 * step;
                result.push((edge_start.0 + offset * dx, edge_start.1 + offset * dy));
            }
            edge_start = *result.last().expect("the boundary contains its start");
        }

        result.pop();
        debug_assert_eq!(result.len(), point_count);
        Ok(result)
    }

    /// Return a densified geographic boundary as longitude/latitude degrees.
    ///
    /// Unlike the upstream compatibility method, this always returns exactly
    /// `4 * points_per_edge - 4` points for every ellipsoidal cell shape.
    /// Points begin at the geographic northwest corner and proceed clockwise.
    pub fn cell_boundary_lonlat(
        &self,
        cell: &CellId,
        points_per_edge: usize,
        interior: bool,
    ) -> Result<Vec<(f64, f64)>> {
        let mut planar = self.cell_boundary_projected(cell, points_per_edge, interior)?;
        let vertices = self.cell_vertices_projected(cell)?;
        let northwest = self.northwest_vertex_index(cell, &vertices)?;
        planar.rotate_left(northwest * (points_per_edge - 1));
        planar
            .into_iter()
            .map(|(x, y)| {
                projection::inverse_in_region(
                    self.ellipsoid,
                    x,
                    y,
                    self.north_square,
                    self.south_square,
                    projection_region(cell.region()),
                )
            })
            .collect()
    }

    /// Return the boundary produced by upstream `Cell.boundary` semantics.
    ///
    /// Geographic quad and cap cells return their four vertices regardless of
    /// `points_per_edge` or `interior`; dart and skew-quad cells use the exact
    /// densified boundary. Planar callers should use [`Self::cell_boundary_projected`].
    pub fn cell_boundary_lonlat_compatible(
        &self,
        cell: &CellId,
        points_per_edge: usize,
        interior: bool,
    ) -> Result<Vec<(f64, f64)>> {
        boundary_point_count(points_per_edge)?;
        if matches!(cell.shape(), CellShape::Quad | CellShape::Cap) {
            self.cell_vertices_lonlat(cell, false)
        } else {
            self.cell_boundary_lonlat(cell, points_per_edge, interior)
        }
    }

    /// Return the edge neighbour in a cardinal direction on the unfolded
    /// rHEALPix plane, applying the required polar-face rotations.
    pub fn planar_neighbor(&self, cell: &CellId, direction: Direction) -> CellId {
        let mut digits = cell.digits().to_vec();
        let mut crosses_border = true;
        for digit in digits.iter_mut().rev() {
            if !crosses_border {
                break;
            }
            let original = *digit;
            *digit = digit_neighbor(original, direction);
            crosses_border = digit_on_border(original, direction);
        }

        let neighbor_face = if crosses_border {
            self.root_neighbor(cell.face(), direction)
        } else {
            cell.face()
        };
        let neighbor = CellId::new(neighbor_face, digits).expect("input cell is already valid");
        let turns = self.neighbor_rotation(cell.face(), neighbor_face);
        neighbor.rotated(turns)
    }

    /// Return all four edge neighbours using geographic direction names.
    ///
    /// Direction names match `Cell.neighbors(plane=False)` in
    /// `rhealpixdggs-py`. Quadrilaterals use north/south/east/west, darts use
    /// two diagonal names, and caps use longitude-ordered indexed names.
    pub fn ellipsoidal_neighbors(
        &self,
        cell: &CellId,
    ) -> Result<Vec<(EllipsoidalDirection, CellId)>> {
        let planar = Direction::ALL.map(|direction| self.planar_neighbor(cell, direction));
        match cell.shape() {
            CellShape::Quad => Ok(vec![
                (EllipsoidalDirection::North, planar[3].clone()),
                (EllipsoidalDirection::South, planar[2].clone()),
                (EllipsoidalDirection::West, planar[0].clone()),
                (EllipsoidalDirection::East, planar[1].clone()),
            ]),
            CellShape::Cap => {
                let mut neighbours = self.neighbor_nuclei(planar, None)?;
                neighbours.sort_by(|left, right| left.0.total_cmp(&right.0));
                Ok(neighbours
                    .into_iter()
                    .enumerate()
                    .map(|(index, (_, _, neighbour))| {
                        let direction = match cell.region() {
                            Region::NorthPolar => EllipsoidalDirection::SouthIndexed(index as u8),
                            Region::SouthPolar => EllipsoidalDirection::NorthIndexed(index as u8),
                            Region::Equatorial => unreachable!("cap cells are polar"),
                        };
                        (direction, neighbour)
                    })
                    .collect())
            }
            CellShape::SkewQuad => {
                let origin = self.cell_to_lonlat(cell)?.0;
                let mut neighbours = self.neighbor_nuclei(planar, Some(origin))?;

                let north_index = index_of_maximum(&neighbours, |entry| entry.1);
                let north = neighbours.remove(north_index).2;
                let south_index = index_of_minimum(&neighbours, |entry| entry.1);
                let south = neighbours.remove(south_index).2;
                let east_index = index_of_maximum(&neighbours, |entry| entry.0);
                let east = neighbours[east_index].2.clone();
                let west_index = index_of_minimum(&neighbours, |entry| entry.0);
                let west = neighbours[west_index].2.clone();

                Ok(vec![
                    (EllipsoidalDirection::North, north),
                    (EllipsoidalDirection::South, south),
                    (EllipsoidalDirection::East, east),
                    (EllipsoidalDirection::West, west),
                ])
            }
            CellShape::Dart => {
                let origin = self.cell_to_lonlat(cell)?.0;
                let mut neighbours = self.neighbor_nuclei(planar, Some(origin))?;
                neighbours.sort_by(|left, right| left.0.total_cmp(&right.0));
                let cells: Vec<_> = neighbours
                    .into_iter()
                    .map(|(_, _, neighbour)| neighbour)
                    .collect();
                Ok(match cell.region() {
                    Region::NorthPolar => vec![
                        (EllipsoidalDirection::West, cells[0].clone()),
                        (EllipsoidalDirection::SouthWest, cells[1].clone()),
                        (EllipsoidalDirection::SouthEast, cells[2].clone()),
                        (EllipsoidalDirection::East, cells[3].clone()),
                    ],
                    Region::SouthPolar => vec![
                        (EllipsoidalDirection::West, cells[0].clone()),
                        (EllipsoidalDirection::NorthWest, cells[1].clone()),
                        (EllipsoidalDirection::NorthEast, cells[2].clone()),
                        (EllipsoidalDirection::East, cells[3].clone()),
                    ],
                    Region::Equatorial => unreachable!("dart cells are polar"),
                })
            }
        }
    }

    /// Return one edge neighbour by geographic direction, or `None` when the
    /// direction name is not applicable to this cell's ellipsoidal shape.
    pub fn ellipsoidal_neighbor(
        &self,
        cell: &CellId,
        direction: EllipsoidalDirection,
    ) -> Result<Option<CellId>> {
        Ok(self
            .ellipsoidal_neighbors(cell)?
            .into_iter()
            .find_map(|(candidate, neighbour)| (candidate == direction).then_some(neighbour)))
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

    fn northwest_vertex_index(&self, cell: &CellId, vertices: &[(f64, f64); 4]) -> Result<usize> {
        match cell.shape() {
            CellShape::Quad | CellShape::Cap => Ok(0),
            CellShape::SkewQuad => {
                let (x, y) = self.cell_to_projected(cell)?;
                let radius = self.ellipsoid.authalic_radius();
                let (triangle, region) = projection::triangle_number(
                    x / radius,
                    y / radius,
                    self.north_square,
                    self.south_square,
                    true,
                );
                let index = match region {
                    projection::Region::NorthPolar => {
                        let offset = (triangle - i32::from(self.north_square)).rem_euclid(4);
                        (4 - offset as usize) % 4
                    }
                    projection::Region::SouthPolar => {
                        (triangle - i32::from(self.south_square)).rem_euclid(4) as usize
                    }
                    projection::Region::Equatorial => {
                        unreachable!("skew quadrilateral cells are polar")
                    }
                };
                Ok(index)
            }
            CellShape::Dart => {
                let mut poleward_index = 0;
                let mut poleward_latitude = f64::NEG_INFINITY;
                for (index, point) in vertices.iter().enumerate() {
                    let (_, latitude) = projection::inverse_in_region(
                        self.ellipsoid,
                        point.0,
                        point.1,
                        self.north_square,
                        self.south_square,
                        projection_region(cell.region()),
                    )?;
                    let magnitude = latitude.abs();
                    if magnitude >= poleward_latitude {
                        poleward_latitude = magnitude;
                        poleward_index = index;
                    }
                }
                Ok(match cell.region() {
                    Region::NorthPolar => poleward_index,
                    Region::SouthPolar => (poleward_index + 1) % 4,
                    Region::Equatorial => unreachable!("dart cells are polar"),
                })
            }
        }
    }

    fn neighbor_nuclei(
        &self,
        neighbours: [CellId; 4],
        relative_to_longitude: Option<f64>,
    ) -> Result<Vec<(f64, f64, CellId)>> {
        neighbours
            .into_iter()
            .map(|neighbour| {
                let (longitude, latitude) = self.cell_to_lonlat(&neighbour)?;
                let longitude = relative_to_longitude
                    .map_or(longitude, |origin| longitude_delta(longitude, origin));
                Ok((longitude, latitude, neighbour))
            })
            .collect()
    }

    fn root_neighbor(&self, face: Face, direction: Direction) -> Face {
        match face {
            Face::O | Face::P | Face::Q | Face::R => match direction {
                Direction::Left => equatorial_face(face.number() - 1 + 3),
                Direction::Right => equatorial_face(face.number() - 1 + 1),
                Direction::Down => Face::S,
                Direction::Up => Face::N,
            },
            Face::N => {
                let offset = match direction {
                    Direction::Down => 0,
                    Direction::Right => 1,
                    Direction::Up => 2,
                    Direction::Left => 3,
                };
                equatorial_face(self.north_square + offset)
            }
            Face::S => {
                let offset = match direction {
                    Direction::Up => 0,
                    Direction::Right => 1,
                    Direction::Down => 2,
                    Direction::Left => 3,
                };
                equatorial_face(self.south_square + offset)
            }
        }
    }

    fn neighbor_rotation(&self, original: Face, neighbor: Face) -> u8 {
        let north_left = self.root_neighbor(Face::N, Direction::Left);
        let north_right = self.root_neighbor(Face::N, Direction::Right);
        let north_up = self.root_neighbor(Face::N, Direction::Up);
        let south_left = self.root_neighbor(Face::S, Direction::Left);
        let south_right = self.root_neighbor(Face::S, Direction::Right);
        let south_down = self.root_neighbor(Face::S, Direction::Down);

        if (original == Face::S && neighbor == south_left)
            || (original == south_right && neighbor == Face::S)
            || (original == Face::N && neighbor == north_right)
            || (original == north_left && neighbor == Face::N)
        {
            1
        } else if (original == Face::S && neighbor == south_down)
            || (original == south_down && neighbor == Face::S)
            || (original == Face::N && neighbor == north_up)
            || (original == north_up && neighbor == Face::N)
        {
            2
        } else if (original == Face::S && neighbor == south_right)
            || (original == south_left && neighbor == Face::S)
            || (original == Face::N && neighbor == north_left)
            || (original == north_right && neighbor == Face::N)
        {
            3
        } else {
            0
        }
    }
}

const fn equatorial_face(index: u8) -> Face {
    match index % 4 {
        0 => Face::O,
        1 => Face::P,
        2 => Face::Q,
        3 => Face::R,
        _ => unreachable!(),
    }
}

const fn projection_region(region: Region) -> projection::Region {
    match region {
        Region::NorthPolar => projection::Region::NorthPolar,
        Region::Equatorial => projection::Region::Equatorial,
        Region::SouthPolar => projection::Region::SouthPolar,
    }
}

const fn cell_region(region: projection::Region) -> Region {
    match region {
        projection::Region::NorthPolar => Region::NorthPolar,
        projection::Region::Equatorial => Region::Equatorial,
        projection::Region::SouthPolar => Region::SouthPolar,
    }
}

const fn digit_on_border(digit: u8, direction: Direction) -> bool {
    match direction {
        Direction::Left => digit % 3 == 0,
        Direction::Right => digit % 3 == 2,
        Direction::Up => digit / 3 == 0,
        Direction::Down => digit / 3 == 2,
    }
}

const fn digit_neighbor(digit: u8, direction: Direction) -> u8 {
    match direction {
        Direction::Left => digit / 3 * 3 + (digit % 3 + 2) % 3,
        Direction::Right => digit / 3 * 3 + (digit % 3 + 1) % 3,
        Direction::Up => ((digit / 3 + 2) % 3) * 3 + digit % 3,
        Direction::Down => ((digit / 3 + 1) % 3) * 3 + digit % 3,
    }
}

fn longitude_delta(longitude: f64, origin: f64) -> f64 {
    (longitude - origin + 180.0).rem_euclid(360.0) - 180.0
}

fn stable_grid_index(fraction: f64, scale: u64) -> u64 {
    let scaled = fraction * scale as f64;
    let nearest = scaled.round();
    let tolerance = 16.0 * f64::EPSILON * scale as f64;
    let stable = if (scaled - nearest).abs() <= tolerance {
        nearest
    } else {
        scaled
    };
    (stable.floor() as u64).min(scale - 1)
}

pub(crate) fn boundary_point_count(points_per_edge: usize) -> Result<usize> {
    if points_per_edge < 2 {
        return Err(Error::InvalidBoundaryPointCount(points_per_edge));
    }
    let count = points_per_edge
        .checked_sub(1)
        .and_then(|value| value.checked_mul(4))
        .ok_or(Error::BoundaryTooLarge(u64::MAX))?;
    if count > MAX_BOUNDARY_POINTS {
        return Err(Error::BoundaryTooLarge(count as u64));
    }
    Ok(count)
}

fn index_of_maximum<T, F>(values: &[T], key: F) -> usize
where
    F: Fn(&T) -> f64,
{
    let mut result = 0;
    for index in 1..values.len() {
        if key(&values[index]) > key(&values[result]) {
            result = index;
        }
    }
    result
}

fn index_of_minimum<T, F>(values: &[T], key: F) -> usize
where
    F: Fn(&T) -> f64,
{
    let mut result = 0;
    for index in 1..values.len() {
        if key(&values[index]) < key(&values[result]) {
            result = index;
        }
    }
    result
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

    #[test]
    fn exact_cell_edges_use_upstream_tie_breaking() {
        let dggs = RhealpixDggs::wgs84_003();
        for (longitude, latitude, resolution, expected) in [
            (10.0, -20.0, 3, "Q616"),
            (10.0, 20.0, 3, "Q070"),
            (20.0, 10.0, 3, "Q320"),
        ] {
            assert_eq!(
                dggs.cell_from_lonlat(longitude, latitude, resolution)
                    .unwrap()
                    .to_string(),
                expected
            );
        }
    }

    #[test]
    fn planar_neighbors_match_upstream_polar_and_wrapping_cases() {
        let dggs = RhealpixDggs::wgs84_003();
        let cases = [
            ("N0", ["R0", "N1", "N3", "Q2"]),
            ("S0", ["R8", "S1", "S3", "O6"]),
            ("N62", ["N61", "N70", "N65", "N38"]),
            ("O0", ["R2", "O1", "O3", "N6"]),
            ("Q888", ["Q887", "R666", "S666", "Q885"]),
            ("R000", ["Q222", "R001", "R003", "N000"]),
        ];
        for (identifier, expected) in cases {
            let cell: CellId = identifier.parse().unwrap();
            let actual =
                Direction::ALL.map(|direction| dggs.planar_neighbor(&cell, direction).to_string());
            assert_eq!(actual, expected, "{identifier}");
        }
    }

    #[test]
    fn planar_neighbors_respect_custom_polar_square_positions() {
        let dggs = RhealpixDggs::new(Ellipsoid::wgs84(), 1, 3);
        let cases = [
            ("N0", ["O0", "N1", "N3", "R2"]),
            ("S0", ["Q8", "S1", "S3", "R6"]),
            ("O0", ["R2", "O1", "O3", "N0"]),
        ];
        for (identifier, expected) in cases {
            let cell: CellId = identifier.parse().unwrap();
            let actual =
                Direction::ALL.map(|direction| dggs.planar_neighbor(&cell, direction).to_string());
            assert_eq!(actual, expected, "{identifier}");
        }
    }

    #[test]
    fn ellipsoidal_neighbors_match_every_upstream_shape() {
        let dggs = RhealpixDggs::wgs84_003();
        let cases: [(&str, [(&str, &str); 4]); 8] = [
            (
                "P2",
                [
                    ("north", "N2"),
                    ("south", "P5"),
                    ("west", "P1"),
                    ("east", "Q0"),
                ],
            ),
            (
                "N",
                [
                    ("south_0", "O"),
                    ("south_1", "P"),
                    ("south_2", "Q"),
                    ("south_3", "R"),
                ],
            ),
            (
                "S4",
                [
                    ("north_0", "S1"),
                    ("north_1", "S5"),
                    ("north_2", "S7"),
                    ("north_3", "S3"),
                ],
            ),
            (
                "N0",
                [
                    ("west", "N1"),
                    ("south_west", "Q2"),
                    ("south_east", "R0"),
                    ("east", "N3"),
                ],
            ),
            (
                "S0",
                [
                    ("west", "S3"),
                    ("north_west", "R8"),
                    ("north_east", "O6"),
                    ("east", "S1"),
                ],
            ),
            (
                "N43",
                [
                    ("north", "N44"),
                    ("south", "N35"),
                    ("east", "N46"),
                    ("west", "N40"),
                ],
            ),
            (
                "S43",
                [
                    ("north", "S35"),
                    ("south", "S44"),
                    ("east", "S40"),
                    ("west", "S46"),
                ],
            ),
            (
                "N62",
                [
                    ("west", "N38"),
                    ("south_west", "N61"),
                    ("south_east", "N65"),
                    ("east", "N70"),
                ],
            ),
        ];

        for (identifier, expected) in cases {
            let cell: CellId = identifier.parse().unwrap();
            let actual: Vec<_> = dggs
                .ellipsoidal_neighbors(&cell)
                .unwrap()
                .into_iter()
                .map(|(direction, neighbour)| (direction.to_string(), neighbour.to_string()))
                .collect();
            let expected: Vec<_> = expected
                .into_iter()
                .map(|(direction, neighbour)| (direction.to_owned(), neighbour.to_owned()))
                .collect();
            assert_eq!(actual, expected, "{identifier}");
        }
    }

    #[test]
    fn ellipsoidal_neighbors_respect_custom_polar_square_positions() {
        let dggs = RhealpixDggs::new(Ellipsoid::wgs84(), 1, 3);
        let cell: CellId = "N0".parse().unwrap();
        let actual: Vec<_> = dggs
            .ellipsoidal_neighbors(&cell)
            .unwrap()
            .into_iter()
            .map(|(direction, neighbour)| (direction.to_string(), neighbour.to_string()))
            .collect();
        assert_eq!(
            actual,
            [
                ("west".to_owned(), "N1".to_owned()),
                ("south_west".to_owned(), "R2".to_owned()),
                ("south_east".to_owned(), "O0".to_owned()),
                ("east".to_owned(), "N3".to_owned()),
            ]
        );
    }

    #[test]
    fn invalid_ellipsoidal_direction_for_shape_returns_none() {
        let dggs = RhealpixDggs::wgs84_003();
        let cell: CellId = "P2".parse().unwrap();
        let direction: EllipsoidalDirection = "north_west".parse().unwrap();
        assert_eq!(dggs.ellipsoidal_neighbor(&cell, direction).unwrap(), None);
    }

    #[test]
    fn geographic_vertices_match_upstream_order_and_dart_trimming() {
        let dggs = RhealpixDggs::wgs84_003();
        let cases: [(&str, &[(f64, f64)]); 5] = [
            (
                "N0",
                &[
                    (90.0, 74.424_006_701_996),
                    (120.0, 41.937_853_910_160),
                    (90.0, 41.937_853_910_160),
                    (60.0, 41.937_853_910_160),
                ],
            ),
            (
                "S0",
                &[
                    (150.0, -41.937_853_910_160),
                    (-180.0, -41.937_853_910_160),
                    (-150.0, -41.937_853_910_160),
                    (-180.0, -74.424_006_701_996),
                ],
            ),
            (
                "N43",
                &[
                    (90.0, 84.823_337_653_191),
                    (-180.0, 84.823_337_653_191),
                    (150.0, 74.424_006_701_996),
                    (120.0, 74.424_006_701_996),
                ],
            ),
            (
                "S43",
                &[
                    (120.0, -74.424_006_701_996),
                    (150.0, -74.424_006_701_996),
                    (-180.0, -84.823_337_653_191),
                    (90.0, -84.823_337_653_191),
                ],
            ),
            (
                "Q77",
                &[
                    (40.0, -31.346_830_117_185_274),
                    (50.0, -31.346_830_117_185_274),
                    (50.0, -41.937_853_910_160_13),
                    (40.0, -41.937_853_910_160_13),
                ],
            ),
        ];
        for (identifier, expected) in cases {
            let cell: CellId = identifier.parse().unwrap();
            let actual = dggs.cell_vertices_lonlat(&cell, false).unwrap();
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.into_iter().zip(expected) {
                assert_close(actual, *expected, 2e-10);
            }
        }

        for identifier in ["N0", "S0", "N62", "S62"] {
            let cell: CellId = identifier.parse().unwrap();
            assert_eq!(dggs.cell_vertices_lonlat(&cell, true).unwrap().len(), 3);
        }
    }

    #[test]
    fn densified_boundaries_match_upstream_examples() {
        let unit = RhealpixDggs::new(Ellipsoid::sphere(1.0).unwrap(), 0, 0);
        let cell: CellId = "N6".parse().unwrap();
        let expected_planar = [
            (-PI, 5.0 * PI / 12.0),
            (-11.0 * PI / 12.0, 5.0 * PI / 12.0),
            (-5.0 * PI / 6.0, 5.0 * PI / 12.0),
            (-5.0 * PI / 6.0, PI / 3.0),
            (-5.0 * PI / 6.0, PI / 4.0),
            (-11.0 * PI / 12.0, PI / 4.0),
            (-PI, PI / 4.0),
            (-PI, PI / 3.0),
        ];
        let actual = unit.cell_boundary_projected(&cell, 3, false).unwrap();
        for (actual, expected) in actual.into_iter().zip(expected_planar) {
            assert_close(actual, expected, 1e-14);
        }

        let wgs84 = RhealpixDggs::wgs84_003();
        let cell: CellId = "N0".parse().unwrap();
        let expected_geographic = [
            (90.0, 74.424_006_701_996),
            (112.5, 58.528_017_482_062_19),
            (120.0, 41.937_853_910_160_14),
            (105.0, 41.937_853_910_160_14),
            (90.0, 41.937_853_910_160_14),
            (75.0, 41.937_853_910_160_14),
            (60.0, 41.937_853_910_160_14),
            (67.5, 58.528_017_482_062_19),
        ];
        let actual = wgs84.cell_boundary_lonlat(&cell, 3, false).unwrap();
        for (actual, expected) in actual.into_iter().zip(expected_geographic) {
            assert_close(actual, expected, 2e-10);
        }
    }

    #[test]
    fn boundary_point_contract_is_exact_and_compatibility_is_explicit() {
        let dggs = RhealpixDggs::wgs84_003();
        for identifier in ["P2", "N", "N0", "N43"] {
            let cell: CellId = identifier.parse().unwrap();
            for points_per_edge in [2, 3, 5] {
                let expected = 4 * points_per_edge - 4;
                assert_eq!(
                    dggs.cell_boundary_projected(&cell, points_per_edge, false)
                        .unwrap()
                        .len(),
                    expected,
                    "projected {identifier}"
                );
                assert_eq!(
                    dggs.cell_boundary_lonlat(&cell, points_per_edge, false)
                        .unwrap()
                        .len(),
                    expected,
                    "geographic {identifier}"
                );
                let compatible = dggs
                    .cell_boundary_lonlat_compatible(&cell, points_per_edge, true)
                    .unwrap();
                let expected_compatible =
                    if matches!(cell.shape(), CellShape::Quad | CellShape::Cap) {
                        4
                    } else {
                        expected
                    };
                assert_eq!(
                    compatible.len(),
                    expected_compatible,
                    "compatible {identifier}"
                );
            }
        }

        let cell: CellId = "P2".parse().unwrap();
        assert_eq!(
            dggs.cell_boundary_projected(&cell, 1, false),
            Err(Error::InvalidBoundaryPointCount(1))
        );
        assert!(matches!(
            dggs.cell_boundary_projected(&cell, usize::MAX, false),
            Err(Error::BoundaryTooLarge(_))
        ));
    }

    #[test]
    fn interior_boundary_is_inset_on_every_projected_edge() {
        let dggs = RhealpixDggs::wgs84_003();
        let cell: CellId = "N62".parse().unwrap();
        let outer = dggs.cell_boundary_projected(&cell, 3, false).unwrap();
        let inner = dggs.cell_boundary_projected(&cell, 3, true).unwrap();
        assert_ne!(outer, inner);

        let vertices = dggs.cell_vertices_projected(&cell).unwrap();
        let (left, top) = vertices[0];
        let (right, bottom) = vertices[2];
        assert!(
            inner
                .iter()
                .all(|&(x, y)| { x > left && x < right && y < top && y > bottom })
        );
    }

    #[test]
    fn every_cell_through_resolution_three_has_valid_geographic_vertices() {
        let dggs = RhealpixDggs::wgs84_003();
        for face_number in 0..6 {
            let root = CellId::new(Face::from_number(face_number).unwrap(), Vec::new()).unwrap();
            for resolution in 0..=3 {
                for cell in root.descendants(resolution).unwrap() {
                    let vertices = dggs.cell_vertices_lonlat(&cell, false).unwrap();
                    assert_eq!(vertices.len(), 4, "{cell}");
                    let trimmed = dggs.cell_vertices_lonlat(&cell, true).unwrap();
                    let expected = if cell.shape() == CellShape::Dart {
                        3
                    } else {
                        4
                    };
                    assert_eq!(trimmed.len(), expected, "{cell}");
                }
            }
        }
    }
}
