//! Dependency-free line, rectangle, and polygon coverage.

use std::collections::BTreeSet;
use std::f64::consts::PI;

use crate::cell::{CellId, CellShape, EllipsoidalDirection, Region, compact_cells};
use crate::dggs::RhealpixDggs;
use crate::error::{Error, Result};
use crate::projection;

const MAX_COVERAGE_CELLS: usize = 10_000_000;
const EPSILON: f64 = 1e-11;
const AREA_EPSILON_MULTIPLIER: f64 = 64.0;

type Point = (f64, f64);

impl RhealpixDggs {
    /// Return rows of cells covering a projected axis-aligned rectangle.
    ///
    /// `upper_left` and `lower_right` are rHEALPix metres. Rows are ordered
    /// top-to-bottom and cells within each row are ordered left-to-right.
    pub fn cells_from_region_projected(
        &self,
        resolution: u8,
        upper_left: Point,
        lower_right: Point,
    ) -> Result<Vec<Vec<CellId>>> {
        validate_projected_point(upper_left)?;
        validate_projected_point(lower_right)?;
        if upper_left.0 > lower_right.0 || upper_left.1 < lower_right.1 {
            return Ok(Vec::new());
        }

        let upper_right = (lower_right.0, upper_left.1);
        let lower_left = (upper_left.0, lower_right.1);
        let Some(upper_left_cell) = projected_cell_or_none(self, upper_left, resolution)? else {
            return Ok(Vec::new());
        };
        let Some(upper_right_cell) = projected_cell_or_none(self, upper_right, resolution)? else {
            return Ok(Vec::new());
        };
        let Some(_lower_left_cell) = projected_cell_or_none(self, lower_left, resolution)? else {
            return Ok(Vec::new());
        };
        let Some(lower_right_cell) = projected_cell_or_none(self, lower_right, resolution)? else {
            return Ok(Vec::new());
        };
        if upper_left_cell == lower_right_cell {
            return Ok(vec![vec![upper_left_cell]]);
        }

        let mut rows = Vec::new();
        let mut row_start = upper_left_cell;
        let mut row_end = upper_right_cell;
        let mut count = 0_usize;
        loop {
            let mut row = Vec::new();
            let mut current = row_start.clone();
            loop {
                push_coverage_cell(&mut row, current.clone(), &mut count)?;
                if current == row_end {
                    break;
                }
                current = self.planar_neighbor(&current, crate::Direction::Right);
            }
            let is_final_row = row_end == lower_right_cell;
            rows.push(row);
            if is_final_row {
                break;
            }
            row_start = self.planar_neighbor(&row_start, crate::Direction::Down);
            row_end = self.planar_neighbor(&row_end, crate::Direction::Down);
        }
        Ok(rows)
    }

    /// Return rows of cells covering a longitude/latitude-aligned region.
    ///
    /// Coordinates are `(longitude, latitude)` degrees. The region must not
    /// cross the antimeridian; callers can split such a region into two. Rows
    /// are north-to-south and cells within a row are west-to-east.
    pub fn cells_from_region_lonlat(
        &self,
        resolution: u8,
        northwest: Point,
        southeast: Point,
    ) -> Result<Vec<Vec<CellId>>> {
        validate_lonlat_point(northwest)?;
        validate_lonlat_point(southeast)?;
        if northwest.0 > southeast.0 || northwest.1 < southeast.1 {
            return Ok(Vec::new());
        }

        let latitude_min = southeast.1;
        let latitude_max = northwest.1;
        let is_north_cap = nearly_equal(northwest.0, -180.0)
            && nearly_equal(northwest.1, 90.0)
            && nearly_equal(southeast.0, -180.0);
        let is_south_cap = nearly_equal(southeast.0, -180.0)
            && nearly_equal(southeast.1, -90.0)
            && nearly_equal(northwest.0, -180.0);
        let (longitude_min, longitude_max) = if is_north_cap || is_south_cap {
            (-180.0, 180.0)
        } else {
            (northwest.0, southeast.0)
        };

        let latitudes = self.cell_latitudes_lonlat(resolution, latitude_min, latitude_max)?;
        let mut rows = Vec::new();
        let mut count = 0_usize;
        for latitude in latitudes.into_iter().rev() {
            let row =
                self.cells_from_parallel(resolution, latitude, longitude_min, longitude_max)?;
            count = checked_coverage_count(count, row.len())?;
            rows.push(row);
        }

        let upper_left_cell = self.cell_from_lonlat(northwest.0, northwest.1, resolution)?;
        if rows
            .first()
            .and_then(|row| row.first())
            .is_none_or(|cell| *cell != upper_left_cell)
        {
            let row =
                self.cells_from_parallel(resolution, latitude_max, longitude_min, longitude_max)?;
            count = checked_coverage_count(count, row.len())?;
            rows.insert(0, row);
        }

        let lower_left_cell = self.cell_from_lonlat(northwest.0, southeast.1, resolution)?;
        if rows
            .last()
            .and_then(|row| row.first())
            .is_none_or(|cell| *cell != lower_left_cell)
        {
            let row =
                self.cells_from_parallel(resolution, latitude_min, longitude_min, longitude_max)?;
            checked_coverage_count(count, row.len())?;
            rows.push(row);
        }
        Ok(rows)
    }

    /// Return cells touched by a projected polyline in path order.
    pub fn cells_from_polyline_projected(
        &self,
        resolution: u8,
        points: &[Point],
    ) -> Result<Vec<CellId>> {
        validate_polyline(points, validate_projected_point)?;
        let mut result = Vec::new();
        for segment in points.windows(2) {
            let upper_left = (
                segment[0].0.min(segment[1].0),
                segment[0].1.max(segment[1].1),
            );
            let lower_right = (
                segment[0].0.max(segment[1].0),
                segment[0].1.min(segment[1].1),
            );
            let candidates = flatten_rows(self.cells_from_region_projected(
                resolution,
                upper_left,
                lower_right,
            )?);
            let mut touched = Vec::new();
            for cell in candidates {
                let vertices = self.cell_vertices_projected(&cell)?;
                let entry = segment_rectangle_entry(
                    segment[0],
                    segment[1],
                    vertices[0].0,
                    vertices[2].0,
                    vertices[2].1,
                    vertices[0].1,
                );
                if let Some(entry) = entry {
                    touched.push((entry, cell));
                }
            }
            append_segment_cells(&mut result, touched)?;
        }
        Ok(result)
    }

    /// Return cells touched by a longitude/latitude polyline in path order.
    ///
    /// Segments are straight in longitude/latitude space, matching upstream
    /// behavior. Longitudes are unwrapped so antimeridian segments take the
    /// short path. Polar cap cells are handled explicitly.
    pub fn cells_from_polyline_lonlat(
        &self,
        resolution: u8,
        points: &[Point],
    ) -> Result<Vec<CellId>> {
        validate_polyline(points, validate_lonlat_point)?;
        let unwrapped = unwrap_longitudes(points, None);
        let mut result = Vec::new();
        for segment in unwrapped.windows(2) {
            let candidates = self.geographic_candidates(
                resolution,
                segment[0].0.min(segment[1].0),
                segment[0].0.max(segment[1].0),
                segment[0].1.min(segment[1].1),
                segment[0].1.max(segment[1].1),
            )?;
            let mut touched = Vec::new();
            for cell in candidates {
                if let Some(entry) = self.geographic_cell_entry(&cell, segment[0], segment[1])? {
                    touched.push((entry, cell));
                }
            }
            append_segment_cells(&mut result, touched)?;
        }
        Ok(result)
    }

    /// Fill a projected polygon using cell-centroid containment.
    ///
    /// Rings contain `(x, y)` metres and do not need to repeat their first
    /// point. Holes use the same ordering. When `compact` is true, complete
    /// sibling groups are recursively replaced by their parents.
    pub fn cells_from_polygon_projected(
        &self,
        resolution: u8,
        exterior: &[Point],
        holes: &[Vec<Point>],
        compact: bool,
    ) -> Result<Vec<CellId>> {
        validate_polygon(exterior, holes, validate_projected_point, false)?;
        let (x_min, x_max, y_min, y_max) = bounds(exterior);
        let candidates = flatten_rows(self.cells_from_region_projected(
            resolution,
            (x_min, y_max),
            (x_max, y_min),
        )?);
        let mut cells = Vec::new();
        for cell in candidates {
            let centroid = self.cell_to_projected(&cell)?;
            if polygon_contains(centroid, exterior, holes) {
                cells.push(cell);
            }
        }
        Ok(if compact { compact_cells(cells) } else { cells })
    }

    /// Fill a longitude/latitude polygon using cell-centroid containment.
    ///
    /// Rings contain `(longitude, latitude)` degrees. Antimeridian-crossing
    /// polygons are unwrapped automatically. Boundary centroids are excluded,
    /// matching upstream/Shapely `contains` semantics.
    pub fn cells_from_polygon_lonlat(
        &self,
        resolution: u8,
        exterior: &[Point],
        holes: &[Vec<Point>],
        compact: bool,
    ) -> Result<Vec<CellId>> {
        validate_polygon(exterior, holes, validate_lonlat_point, true)?;
        let exterior = unwrap_longitudes(exterior, None);
        let anchor = longitude_anchor(&exterior);
        let holes: Vec<_> = holes
            .iter()
            .map(|hole| unwrap_longitudes(hole, Some(anchor)))
            .collect();
        let (longitude_min, longitude_max, latitude_min, latitude_max) = bounds(&exterior);
        let candidates = self.geographic_candidates(
            resolution,
            longitude_min,
            longitude_max,
            latitude_min,
            latitude_max,
        )?;
        let mut cells = Vec::new();
        for cell in candidates {
            let (longitude, latitude) = self.cell_centroid_lonlat(&cell)?;
            let centroid = (longitude_near(longitude, anchor), latitude);
            if polygon_contains(centroid, &exterior, &holes) {
                cells.push(cell);
            }
        }
        Ok(if compact { compact_cells(cells) } else { cells })
    }

    /// Return the upstream-compatible ellipsoidal centroid of a cell.
    ///
    /// Polar darts and skew quadrilaterals use fixed Gauss-Legendre
    /// integration over the projected cell square, avoiding a runtime
    /// dependency on SciPy.
    pub fn cell_centroid_lonlat(&self, cell: &CellId) -> Result<Point> {
        let nucleus = self.cell_to_lonlat(cell)?;
        let vertices = self.cell_vertices_lonlat(cell, false)?;
        match cell.shape() {
            CellShape::Cap => Ok(nucleus),
            CellShape::Quad => Ok((
                nucleus.0,
                vertices.iter().map(|point| point.1).sum::<f64>() / 4.0,
            )),
            CellShape::Dart | CellShape::SkewQuad => {
                let planar = self.cell_vertices_projected(cell)?;
                let x_mid = (planar[0].0 + planar[2].0) / 2.0;
                let y_mid = (planar[0].1 + planar[2].1) / 2.0;
                let half_width = (planar[2].0 - planar[0].0) / 2.0;
                let mut latitude = 0.0;
                let mut longitude = 0.0;
                for (x_node, x_weight) in GAUSS_NODES.into_iter().zip(GAUSS_WEIGHTS) {
                    for (y_node, y_weight) in GAUSS_NODES.into_iter().zip(GAUSS_WEIGHTS) {
                        let point = projection::inverse_in_region(
                            self.ellipsoid(),
                            x_mid + half_width * x_node,
                            y_mid + half_width * y_node,
                            self.north_square(),
                            self.south_square(),
                            coverage_projection_region(cell.region()),
                        )?;
                        let weight = x_weight * y_weight / 4.0;
                        latitude += weight * point.1;
                        longitude += weight * longitude_near(point.0, nucleus.0);
                    }
                }
                if cell.shape() == CellShape::Dart {
                    longitude = nucleus.0;
                }
                Ok((wrap_longitude(longitude), latitude))
            }
        }
    }

    fn cell_latitudes_lonlat(
        &self,
        resolution: u8,
        latitude_min: f64,
        latitude_max: f64,
    ) -> Result<Vec<f64>> {
        if latitude_min > latitude_max {
            return Ok(Vec::new());
        }
        let radius = self.ellipsoid().authalic_radius();
        let y_min = projection::latitude_to_healpix_y(self.ellipsoid(), latitude_min)?;
        let y_max = projection::latitude_to_healpix_y(self.ellipsoid(), latitude_max)?;
        let width = self.cell_width(resolution)?;
        let mut y = -radius * PI / 2.0 + width;
        if y <= y_min {
            let difference = y_min - y;
            y = (y + (difference / width).ceil() * width).max(y + width);
        }
        let mut result = Vec::new();
        while y < y_max {
            if result.len() >= MAX_COVERAGE_CELLS {
                return Err(Error::ExpansionTooLarge(result.len() as u64 + 1));
            }
            result.push(projection::healpix_y_to_latitude(self.ellipsoid(), y)?);
            y += width;
        }
        Ok(result)
    }

    fn cells_from_parallel(
        &self,
        resolution: u8,
        latitude: f64,
        longitude_min: f64,
        longitude_max: f64,
    ) -> Result<Vec<CellId>> {
        if longitude_min > longitude_max {
            return Ok(Vec::new());
        }
        let start = self.cell_from_lonlat(longitude_min, latitude, resolution)?;
        let mut end = self.cell_from_lonlat(longitude_max, latitude, resolution)?;
        if start == end {
            if start.shape() == CellShape::Cap || longitude_max - longitude_min < 90.0 {
                return Ok(vec![start]);
            }
            end = self
                .ellipsoidal_neighbor(&start, EllipsoidalDirection::West)?
                .ok_or_else(|| {
                    Error::InvalidGeometry("parallel cannot leave cap cell".to_owned())
                })?;
        }
        let mut result = Vec::new();
        let mut current = start;
        loop {
            if result.len() >= MAX_COVERAGE_CELLS {
                return Err(Error::ExpansionTooLarge(result.len() as u64 + 1));
            }
            result.push(current.clone());
            if current == end {
                break;
            }
            current = self
                .ellipsoidal_neighbor(&current, EllipsoidalDirection::East)?
                .ok_or_else(|| Error::InvalidGeometry("parallel entered cap cell".to_owned()))?;
        }
        Ok(result)
    }

    fn geographic_candidates(
        &self,
        resolution: u8,
        longitude_min: f64,
        longitude_max: f64,
        latitude_min: f64,
        latitude_max: f64,
    ) -> Result<Vec<CellId>> {
        let mut candidates = BTreeSet::new();
        for (west, east) in longitude_intervals(longitude_min, longitude_max) {
            for row in self.cells_from_region_lonlat(
                resolution,
                (west, latitude_max),
                (east, latitude_min),
            )? {
                for cell in row {
                    candidates.insert(cell);
                    if candidates.len() > MAX_COVERAGE_CELLS {
                        return Err(Error::ExpansionTooLarge(candidates.len() as u64));
                    }
                }
            }
        }
        Ok(candidates.into_iter().collect())
    }

    fn geographic_cell_entry(
        &self,
        cell: &CellId,
        start: Point,
        end: Point,
    ) -> Result<Option<f64>> {
        let vertices = self.cell_vertices_lonlat(cell, cell.shape() == CellShape::Dart)?;
        if cell.shape() == CellShape::Cap {
            let boundary_latitude = match cell.region() {
                Region::NorthPolar => vertices
                    .iter()
                    .map(|point| point.1)
                    .fold(f64::INFINITY, f64::min),
                Region::SouthPolar => vertices
                    .iter()
                    .map(|point| point.1)
                    .fold(f64::NEG_INFINITY, f64::max),
                Region::Equatorial => unreachable!("cap cells are polar"),
            };
            return Ok(cap_entry(start, end, boundary_latitude, cell.region()));
        }

        let anchor = (start.0 + end.0) / 2.0;
        let ring = unwrap_longitudes(&vertices, Some(anchor));
        let mut entry = None;
        if point_location(start, &ring) != PointLocation::Outside {
            entry = Some(0.0);
        }
        for index in 0..ring.len() {
            if let Some(value) = segment_intersection_parameter(
                start,
                end,
                ring[index],
                ring[(index + 1) % ring.len()],
            ) {
                entry = Some(entry.map_or(value, |current: f64| current.min(value)));
            }
        }
        Ok(entry)
    }
}

const GAUSS_NODES: [f64; 8] = [
    -0.960_289_856_497_536_3,
    -0.796_666_477_413_626_7,
    -0.525_532_409_916_329,
    -0.183_434_642_495_649_8,
    0.183_434_642_495_649_8,
    0.525_532_409_916_329,
    0.796_666_477_413_626_7,
    0.960_289_856_497_536_3,
];

const GAUSS_WEIGHTS: [f64; 8] = [
    0.101_228_536_290_376_3,
    0.222_381_034_453_374_5,
    0.313_706_645_877_887_3,
    0.362_683_783_378_362,
    0.362_683_783_378_362,
    0.313_706_645_877_887_3,
    0.222_381_034_453_374_5,
    0.101_228_536_290_376_3,
];

fn coverage_projection_region(region: Region) -> projection::Region {
    match region {
        Region::NorthPolar => projection::Region::NorthPolar,
        Region::Equatorial => projection::Region::Equatorial,
        Region::SouthPolar => projection::Region::SouthPolar,
    }
}

fn validate_projected_point(point: Point) -> Result<()> {
    if point.0.is_finite() && point.1.is_finite() {
        Ok(())
    } else {
        Err(Error::InvalidCoordinate(
            "projected coordinates must be finite".to_owned(),
        ))
    }
}

fn validate_lonlat_point(point: Point) -> Result<()> {
    if !point.0.is_finite() || !point.1.is_finite() {
        return Err(Error::InvalidCoordinate(
            "longitude and latitude must be finite".to_owned(),
        ));
    }
    if !(-90.0..=90.0).contains(&point.1) {
        return Err(Error::InvalidCoordinate(format!(
            "latitude {} is outside [-90, 90]",
            point.1
        )));
    }
    Ok(())
}

fn validate_polyline(points: &[Point], validator: fn(Point) -> Result<()>) -> Result<()> {
    if points.len() < 2 {
        return Err(Error::InvalidGeometry(
            "a polyline requires at least two points".to_owned(),
        ));
    }
    points.iter().try_for_each(|point| validator(*point))
}

fn validate_polygon(
    exterior: &[Point],
    holes: &[Vec<Point>],
    validator: fn(Point) -> Result<()>,
    unwrap: bool,
) -> Result<()> {
    validate_ring(exterior, validator, unwrap, "exterior")?;
    for (index, hole) in holes.iter().enumerate() {
        validate_ring(hole, validator, unwrap, &format!("hole {index}"))?;
    }
    Ok(())
}

fn validate_ring(
    ring: &[Point],
    validator: fn(Point) -> Result<()>,
    unwrap: bool,
    name: &str,
) -> Result<()> {
    if ring.len() < 3 {
        return Err(Error::InvalidGeometry(format!(
            "{name} ring requires at least three points"
        )));
    }
    ring.iter().try_for_each(|point| validator(*point))?;
    let values = if unwrap {
        unwrap_longitudes(ring, None)
    } else {
        ring.to_vec()
    };
    if signed_area(&values).abs() <= area_tolerance(&values) {
        return Err(Error::InvalidGeometry(format!("{name} ring has zero area")));
    }
    Ok(())
}

fn projected_cell_or_none(
    dggs: &RhealpixDggs,
    point: Point,
    resolution: u8,
) -> Result<Option<CellId>> {
    match dggs.cell_from_projected(point.0, point.1, resolution) {
        Ok(cell) => Ok(Some(cell)),
        Err(Error::OutsideProjection) => Ok(None),
        Err(error) => Err(error),
    }
}

fn checked_coverage_count(current: usize, additional: usize) -> Result<usize> {
    let count = current
        .checked_add(additional)
        .ok_or(Error::ExpansionTooLarge(u64::MAX))?;
    if count > MAX_COVERAGE_CELLS {
        return Err(Error::ExpansionTooLarge(count as u64));
    }
    Ok(count)
}

fn push_coverage_cell(cells: &mut Vec<CellId>, cell: CellId, count: &mut usize) -> Result<()> {
    *count = checked_coverage_count(*count, 1)?;
    cells.push(cell);
    Ok(())
}

fn flatten_rows(rows: Vec<Vec<CellId>>) -> Vec<CellId> {
    rows.into_iter().flatten().collect()
}

fn append_segment_cells(result: &mut Vec<CellId>, mut touched: Vec<(f64, CellId)>) -> Result<()> {
    touched.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    for (_, cell) in touched {
        if result.last() != Some(&cell) {
            if result.len() >= MAX_COVERAGE_CELLS {
                return Err(Error::ExpansionTooLarge(result.len() as u64 + 1));
            }
            result.push(cell);
        }
    }
    Ok(())
}

fn bounds(points: &[Point]) -> (f64, f64, f64, f64) {
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for &(x, y) in points {
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
    }
    (x_min, x_max, y_min, y_max)
}

fn signed_area(ring: &[Point]) -> f64 {
    let Some(&origin) = ring.first() else {
        return 0.0;
    };

    // Triangulate relative to a local origin. The area is translation
    // invariant, while the smaller local coordinates avoid cancellation when
    // a tiny ring is located at a large absolute longitude or projected x.
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for edge in ring[1..].windows(2) {
        let term = cross(subtract(edge[0], origin), subtract(edge[1], origin));
        let next = sum + term;
        // Neumaier summation also limits cancellation between triangle terms
        // for concave rings without changing the orientation sign.
        compensation += if sum.abs() >= term.abs() {
            (sum - next) + term
        } else {
            (term - next) + sum
        };
        sum = next;
    }
    (sum + compensation) / 2.0
}

fn area_tolerance(ring: &[Point]) -> f64 {
    let (x_min, x_max, y_min, y_max) = bounds(ring);
    let span = (x_max - x_min).max(y_max - y_min);
    AREA_EPSILON_MULTIPLIER * f64::EPSILON * span * span
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PointLocation {
    Outside,
    Inside,
    Boundary,
}

fn point_location(point: Point, ring: &[Point]) -> PointLocation {
    let mut inside = false;
    for index in 0..ring.len() {
        let start = ring[index];
        let end = ring[(index + 1) % ring.len()];
        if point_on_segment(point, start, end) {
            return PointLocation::Boundary;
        }
        if (start.1 > point.1) != (end.1 > point.1) {
            let crossing_x = (end.0 - start.0) * (point.1 - start.1) / (end.1 - start.1) + start.0;
            if point.0 < crossing_x {
                inside = !inside;
            }
        }
    }
    if inside {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}

fn polygon_contains(point: Point, exterior: &[Point], holes: &[Vec<Point>]) -> bool {
    point_location(point, exterior) == PointLocation::Inside
        && holes
            .iter()
            .all(|hole| point_location(point, hole) == PointLocation::Outside)
}

fn point_on_segment(point: Point, start: Point, end: Point) -> bool {
    let cross = cross(subtract(end, start), subtract(point, start));
    let edge_scale = (end.0 - start.0)
        .abs()
        .max((end.1 - start.1).abs())
        .max(1.0);
    let coordinate_scale = point
        .0
        .abs()
        .max(point.1.abs())
        .max(start.0.abs())
        .max(start.1.abs())
        .max(end.0.abs())
        .max(end.1.abs())
        .max(1.0);
    let tolerance = 4.0 * f64::EPSILON * edge_scale * coordinate_scale;
    cross.abs() <= tolerance
        && point.0 >= start.0.min(end.0) - EPSILON
        && point.0 <= start.0.max(end.0) + EPSILON
        && point.1 >= start.1.min(end.1) - EPSILON
        && point.1 <= start.1.max(end.1) + EPSILON
}

fn segment_intersection_parameter(
    line_start: Point,
    line_end: Point,
    edge_start: Point,
    edge_end: Point,
) -> Option<f64> {
    let line = subtract(line_end, line_start);
    let edge = subtract(edge_end, edge_start);
    let offset = subtract(edge_start, line_start);
    let denominator = cross(line, edge);
    if denominator.abs() <= EPSILON {
        if cross(offset, line).abs() > EPSILON {
            return None;
        }
        let length_squared = line.0 * line.0 + line.1 * line.1;
        if length_squared <= EPSILON {
            return point_on_segment(line_start, edge_start, edge_end).then_some(0.0);
        }
        let first = dot(offset, line) / length_squared;
        let second = dot(subtract(edge_end, line_start), line) / length_squared;
        let entry = first.min(second).max(0.0);
        let exit = first.max(second).min(1.0);
        return (entry <= exit + EPSILON).then_some(entry.clamp(0.0, 1.0));
    }
    let line_parameter = cross(offset, edge) / denominator;
    let edge_parameter = cross(offset, line) / denominator;
    if (-EPSILON..=1.0 + EPSILON).contains(&line_parameter)
        && (-EPSILON..=1.0 + EPSILON).contains(&edge_parameter)
    {
        Some(line_parameter.clamp(0.0, 1.0))
    } else {
        None
    }
}

fn segment_rectangle_entry(
    start: Point,
    end: Point,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
) -> Option<f64> {
    let delta = subtract(end, start);
    let mut entry: f64 = 0.0;
    let mut exit: f64 = 1.0;
    for (direction, distance) in [
        (-delta.0, start.0 - x_min),
        (delta.0, x_max - start.0),
        (-delta.1, start.1 - y_min),
        (delta.1, y_max - start.1),
    ] {
        if direction.abs() <= EPSILON {
            if distance < -EPSILON {
                return None;
            }
        } else {
            let parameter = distance / direction;
            if direction < 0.0 {
                exit = exit.min(parameter);
            } else {
                entry = entry.max(parameter);
            }
            if entry > exit + EPSILON {
                return None;
            }
        }
    }
    Some(entry.clamp(0.0, 1.0))
}

fn cap_entry(start: Point, end: Point, boundary: f64, region: Region) -> Option<f64> {
    let contains = |latitude: f64| match region {
        Region::NorthPolar => latitude >= boundary - EPSILON,
        Region::SouthPolar => latitude <= boundary + EPSILON,
        Region::Equatorial => false,
    };
    if contains(start.1) {
        return Some(0.0);
    }
    if !contains(end.1) || nearly_equal(start.1, end.1) {
        return None;
    }
    Some(((boundary - start.1) / (end.1 - start.1)).clamp(0.0, 1.0))
}

fn subtract(left: Point, right: Point) -> Point {
    (left.0 - right.0, left.1 - right.1)
}

fn cross(left: Point, right: Point) -> f64 {
    left.0 * right.1 - left.1 * right.0
}

fn dot(left: Point, right: Point) -> f64 {
    left.0 * right.0 + left.1 * right.1
}

fn unwrap_longitudes(points: &[Point], anchor: Option<f64>) -> Vec<Point> {
    let mut result = Vec::with_capacity(points.len());
    let first = anchor.map_or(points[0].0, |anchor| longitude_near(points[0].0, anchor));
    result.push((first, points[0].1));
    for point in &points[1..] {
        let previous = result.last().expect("the first point was inserted").0;
        result.push((longitude_near(point.0, previous), point.1));
    }
    result
}

fn longitude_near(longitude: f64, anchor: f64) -> f64 {
    anchor + (longitude - anchor + 180.0).rem_euclid(360.0) - 180.0
}

fn wrap_longitude(longitude: f64) -> f64 {
    (longitude + 180.0).rem_euclid(360.0) - 180.0
}

fn longitude_anchor(points: &[Point]) -> f64 {
    let (minimum, maximum, _, _) = bounds(points);
    (minimum + maximum) / 2.0
}

fn longitude_intervals(minimum: f64, maximum: f64) -> Vec<(f64, f64)> {
    if maximum - minimum >= 360.0 - EPSILON {
        return vec![(-180.0, 180.0)];
    }
    let first_tile = ((minimum + 180.0) / 360.0).floor() as i32;
    let last_tile = ((maximum + 180.0) / 360.0).floor() as i32;
    let mut intervals = Vec::new();
    for tile in first_tile..=last_tile {
        let offset = f64::from(tile) * 360.0;
        let west = minimum.max(-180.0 + offset) - offset;
        let east = maximum.min(180.0 + offset) - offset;
        if west <= east + EPSILON {
            intervals.push((west.clamp(-180.0, 180.0), east.clamp(-180.0, 180.0)));
        }
    }
    intervals
}

fn nearly_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ellipsoid;

    fn names(rows: Vec<Vec<CellId>>) -> Vec<Vec<String>> {
        rows.into_iter()
            .map(|row| row.into_iter().map(|cell| cell.to_string()).collect())
            .collect()
    }

    #[test]
    fn geographic_regions_match_upstream_doctests() {
        let dggs = RhealpixDggs::wgs84_003();
        assert_eq!(
            names(
                dggs.cells_from_region_lonlat(1, (0.0, 60.0), (90.0, 0.0))
                    .unwrap()
            ),
            [
                vec!["N2", "N1", "N0"],
                vec!["Q0", "Q1", "Q2", "R0"],
                vec!["Q3", "Q4", "Q5", "R3"],
            ]
        );
        assert_eq!(
            names(
                dggs.cells_from_region_lonlat(1, (-180.0, -36.0), (-180.0, -90.0))
                    .unwrap()
            ),
            [
                vec![
                    "O6", "O7", "O8", "P6", "P7", "P8", "Q6", "Q7", "Q8", "R6", "R7", "R8",
                ],
                vec!["S0", "S1", "S2", "S5", "S8", "S7", "S6", "S3"],
                vec!["S4"],
            ]
        );
    }

    #[test]
    fn planar_region_matches_upstream_doctest() {
        let dggs = RhealpixDggs::wgs84_003();
        let radius = dggs.ellipsoid().authalic_radius();
        assert_eq!(
            names(
                dggs.cells_from_region_projected(
                    1,
                    (-0.1 * radius, radius * PI / 4.0),
                    (0.1 * radius, -radius * PI / 4.0),
                )
                .unwrap()
            ),
            [vec!["P2", "Q0"], vec!["P5", "Q3"], vec!["P8", "Q6"],]
        );
    }

    #[test]
    fn geographic_line_matches_upstream_examples_and_crosses_antimeridian() {
        let dggs = RhealpixDggs::wgs84_003();
        let line = dggs
            .cells_from_polyline_lonlat(3, &[(-89.669_615, 86.549_596), (-134.0, 86.0)])
            .unwrap();
        assert_eq!(
            line.into_iter()
                .map(|cell| cell.to_string())
                .collect::<Vec<_>>(),
            ["N448", "N447"]
        );

        let crossed = dggs
            .cells_from_polyline_lonlat(3, &[(179.0, 0.0), (-179.0, 0.0)])
            .unwrap();
        assert!(crossed.len() < 5);
        assert_eq!(crossed.first().unwrap().face(), crate::Face::R);
        assert_eq!(crossed.last().unwrap().face(), crate::Face::O);

        let polar = dggs
            .cells_from_polyline_lonlat(2, &[(-170.0, 70.0), (0.0, 89.0), (170.0, 70.0)])
            .unwrap();
        assert!(polar.iter().any(|cell| cell.to_string() == "N44"));
    }

    #[test]
    fn polygon_fill_matches_upstream_doctest_and_holes_exclude_centroids() {
        let dggs = RhealpixDggs::wgs84_003();
        let exterior = [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)];
        let cells = dggs
            .cells_from_polygon_lonlat(5, &exterior, &[], false)
            .unwrap();
        assert_eq!(
            cells
                .into_iter()
                .map(|cell| cell.to_string())
                .collect::<Vec<_>>(),
            [
                "Q33303", "Q33304", "Q33305", "Q33306", "Q33307", "Q33308", "Q33330", "Q33331",
                "Q33332",
            ]
        );

        let without_center = dggs
            .cells_from_polygon_lonlat(
                5,
                &exterior,
                &[vec![(0.3, 0.3), (0.3, 0.7), (0.7, 0.7), (0.7, 0.3)]],
                false,
            )
            .unwrap();
        assert!(without_center.len() < 9);
    }

    #[test]
    fn polygon_validation_rejects_a_collinear_ring() {
        let dggs = RhealpixDggs::wgs84_003();
        let step = 2.0_f64.powi(-20);
        let collinear = [
            (175.0, -40.0),
            (175.0 + step, -40.0 + step),
            (175.0 + 2.0 * step, -40.0 + 2.0 * step),
        ];

        let error = dggs
            .cells_from_polygon_lonlat(8, &collinear, &[], false)
            .unwrap_err();
        assert!(matches!(
            error,
            Error::InvalidGeometry(message) if message == "exterior ring has zero area"
        ));
    }

    #[test]
    fn polygon_validation_accepts_a_tiny_nz_ring_in_both_orientations() {
        let dggs = RhealpixDggs::wgs84_003();
        let exterior = [
            (175.0, -40.0),
            (175.000_000_1, -40.0),
            (175.000_000_1, -39.999_999_9),
            (175.0, -39.999_999_9),
        ];
        let reversed: Vec<_> = exterior.iter().copied().rev().collect();

        let forward_area = signed_area(&exterior);
        let reverse_area = signed_area(&reversed);
        assert!(forward_area > area_tolerance(&exterior));
        assert!(reverse_area < -area_tolerance(&reversed));
        assert_eq!(forward_area, -reverse_area);
        dggs.cells_from_polygon_lonlat(8, &exterior, &[], false)
            .unwrap();
        dggs.cells_from_polygon_lonlat(8, &reversed, &[], false)
            .unwrap();
    }

    #[test]
    fn polygon_validation_accepts_a_thin_bisection_sliver() {
        let dggs = RhealpixDggs::wgs84_003();
        let sliver = [
            (175.0, -40.0),
            (175.001, -40.0),
            (175.001, -39.999_999_999_9),
            (175.0, -39.999_999_999_9),
        ];

        assert!(signed_area(&sliver).abs() > area_tolerance(&sliver));
        dggs.cells_from_polygon_lonlat(8, &sliver, &[], false)
            .unwrap();
    }

    #[test]
    fn polygon_area_validation_is_location_and_scale_invariant() {
        let dggs = RhealpixDggs::wgs84_003();
        let locations = [
            (-179.0, -80.0),
            (-120.0, 45.0),
            (0.0, 0.0),
            (120.0, -45.0),
            (179.0, 80.0),
        ];

        for exponent in [10, 20, 30, 40] {
            let width = 2.0_f64.powi(-exponent);
            let height = 2.0_f64.powi(-exponent - 2);
            let expected_area = width * height;

            for (longitude, latitude) in locations {
                let exterior = [
                    (longitude, latitude),
                    (longitude + width, latitude),
                    (longitude + width, latitude + height),
                    (longitude, latitude + height),
                ];
                let reversed: Vec<_> = exterior.iter().copied().rev().collect();
                let forward_area = signed_area(&exterior);
                let reverse_area = signed_area(&reversed);
                let roundoff = expected_area * 8.0 * f64::EPSILON;

                assert!(forward_area > area_tolerance(&exterior));
                assert!(reverse_area < -area_tolerance(&reversed));
                assert!((forward_area - expected_area).abs() <= roundoff);
                assert!((forward_area + reverse_area).abs() <= roundoff);

                let forward = dggs
                    .cells_from_polygon_lonlat(8, &exterior, &[], false)
                    .unwrap();
                let reverse = dggs
                    .cells_from_polygon_lonlat(8, &reversed, &[], false)
                    .unwrap();
                assert_eq!(forward, reverse);
            }
        }
    }

    #[test]
    fn polygon_validation_accepts_tiny_antimeridian_and_polar_rings() {
        let dggs = RhealpixDggs::wgs84_003();
        let step = 2.0_f64.powi(-20);
        let cases = [
            [
                (180.0 - step, -10.0),
                (-180.0 + step, -10.0),
                (-180.0 + step, -10.0 + step),
                (180.0 - step, -10.0 + step),
            ],
            [
                (45.0, 90.0 - 4.0 * step),
                (45.0 + step, 90.0 - 4.0 * step),
                (45.0 + step, 90.0 - 3.0 * step),
                (45.0, 90.0 - 3.0 * step),
            ],
            [
                (-45.0, -90.0 + 3.0 * step),
                (-45.0 + step, -90.0 + 3.0 * step),
                (-45.0 + step, -90.0 + 4.0 * step),
                (-45.0, -90.0 + 4.0 * step),
            ],
        ];

        for exterior in cases {
            let reversed: Vec<_> = exterior.iter().copied().rev().collect();
            let forward = dggs
                .cells_from_polygon_lonlat(8, &exterior, &[], false)
                .unwrap();
            let reverse = dggs
                .cells_from_polygon_lonlat(8, &reversed, &[], false)
                .unwrap();
            assert_eq!(forward, reverse);
        }
    }

    #[test]
    fn centroid_integration_stays_inside_its_cell() {
        let dggs = RhealpixDggs::new(Ellipsoid::wgs84(), 1, 3);
        for identifier in ["P2", "N", "N0", "N43", "S62"] {
            let cell: CellId = identifier.parse().unwrap();
            let centroid = dggs.cell_centroid_lonlat(&cell).unwrap();
            assert_eq!(
                dggs.cell_from_lonlat(centroid.0, centroid.1, cell.resolution())
                    .unwrap(),
                cell,
                "{identifier} {centroid:?}"
            );
        }
    }
}
