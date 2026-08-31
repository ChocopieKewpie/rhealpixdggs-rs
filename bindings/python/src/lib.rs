//! PyO3 bindings for the dependency-free `rhealpixdggs` core.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::str::FromStr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rhealpixdggs::{
    BOUNDARY_PARALLEL_THRESHOLD, CellId, Direction, Ellipsoid, EllipsoidalDirection, Error,
    MAX_RESOLUTION, POINT_PARALLEL_THRESHOLD, REGION_PARALLEL_THRESHOLD, Region, RhealpixDggs,
    compact_cells as compact_core, parallelism_available, uncompact_cells as uncompact_core,
};

fn value_error(error: Error) -> PyErr {
    PyValueError::new_err(error.to_string())
}

fn parse_cell(value: &str) -> PyResult<CellId> {
    CellId::from_str(value).map_err(value_error)
}

fn parse_grid_distance(value: i64) -> PyResult<u32> {
    u32::try_from(value)
        .map_err(|_| PyValueError::new_err("grid distance k must be a non-negative 32-bit integer"))
}

/// Convert latitude/longitude degrees to a WGS84_003 cell ID.
#[pyfunction]
fn latlng_to_cell(latitude: f64, longitude: f64, resolution: u8) -> PyResult<String> {
    RhealpixDggs::wgs84_003()
        .cell_from_lonlat(longitude, latitude, resolution)
        .map(|cell| cell.to_string())
        .map_err(value_error)
}

/// Convert many `(latitude, longitude)` pairs to WGS84_003 cell IDs.
#[pyfunction]
fn latlngs_to_cells(
    py: Python<'_>,
    coordinates: Vec<(f64, f64)>,
    resolution: u8,
) -> PyResult<Vec<String>> {
    let dggs = RhealpixDggs::wgs84_003();
    let use_parallel = coordinates.len() >= POINT_PARALLEL_THRESHOLD;
    py.detach(move || {
        let coordinates: Vec<_> = coordinates
            .into_iter()
            .map(|(latitude, longitude)| (longitude, latitude))
            .collect();
        dggs.cells_from_lonlats_bulk(&coordinates, resolution, use_parallel)
            .map(|cells| cells.into_iter().map(|cell| cell.to_string()).collect())
    })
    .map_err(value_error)
}

fn use_parallel(requested: Option<bool>, count: usize, threshold: usize) -> bool {
    parallelism_available() && requested.unwrap_or(count >= threshold)
}

fn checked_chunks(data: &[u8], width: usize, label: &str) -> PyResult<usize> {
    if data.len() % width != 0 {
        return Err(PyValueError::new_err(format!(
            "{label} byte length must be divisible by {width}; got {}",
            data.len()
        )));
    }
    Ok(data.len() / width)
}

fn f64_at(chunk: &[u8], offset: usize) -> f64 {
    f64::from_le_bytes(
        chunk[offset..offset + 8]
            .try_into()
            .expect("the caller validated the chunk width"),
    )
}

fn encode_cells(cells: &[CellId]) -> Vec<u8> {
    let mut output = Vec::with_capacity(cells.len() * 8);
    for cell in cells {
        output.extend_from_slice(&cell.to_u64().to_le_bytes());
    }
    output
}

fn decode_cells(data: &[u8]) -> std::result::Result<Vec<CellId>, Error> {
    data.chunks_exact(8)
        .map(|chunk| {
            CellId::from_u64(u64::from_le_bytes(
                chunk.try_into().expect("chunks are exactly eight bytes"),
            ))
        })
        .collect()
}

/// Rust buffer backend for NumPy latitude/longitude to integer-cell batches.
#[pyfunction(name = "_latlngs_to_cells_buffer", signature = (data, resolution, parallel=None))]
fn latlngs_to_cells_buffer(
    py: Python<'_>,
    data: Py<PyBytes>,
    resolution: u8,
    parallel: Option<bool>,
) -> PyResult<Py<PyBytes>> {
    let data = data.as_bytes(py);
    let count = checked_chunks(data, 16, "coordinate")?;
    let parallel = use_parallel(parallel, count, POINT_PARALLEL_THRESHOLD);
    let output = py
        .detach(|| {
            let coordinates: Vec<_> = data
                .chunks_exact(16)
                .map(|chunk| (f64_at(chunk, 8), f64_at(chunk, 0)))
                .collect();
            RhealpixDggs::wgs84_003()
                .cells_from_lonlats_bulk(&coordinates, resolution, parallel)
                .map(|cells| encode_cells(&cells))
        })
        .map_err(value_error)?;
    Ok(PyBytes::new(py, &output).unbind())
}

/// Rust buffer backend for integer-cell to latitude/longitude batches.
#[pyfunction(name = "_cells_to_latlngs_buffer", signature = (data, parallel=None))]
fn cells_to_latlngs_buffer(
    py: Python<'_>,
    data: Py<PyBytes>,
    parallel: Option<bool>,
) -> PyResult<Py<PyBytes>> {
    let data = data.as_bytes(py);
    let count = checked_chunks(data, 8, "cell")?;
    let parallel = use_parallel(parallel, count, POINT_PARALLEL_THRESHOLD);
    let output = py
        .detach(|| {
            let cells = decode_cells(data)?;
            let points = RhealpixDggs::wgs84_003().lonlats_from_cells_bulk(&cells, parallel)?;
            let mut output = Vec::with_capacity(points.len() * 16);
            for (longitude, latitude) in points {
                output.extend_from_slice(&latitude.to_le_bytes());
                output.extend_from_slice(&longitude.to_le_bytes());
            }
            Ok::<_, Error>(output)
        })
        .map_err(value_error)?;
    Ok(PyBytes::new(py, &output).unbind())
}

/// Rust buffer backend for integer-cell to fixed boundary batches.
#[pyfunction(name = "_cells_to_boundaries_buffer", signature = (data, points_per_edge=2, interior=false, parallel=None))]
fn cells_to_boundaries_buffer(
    py: Python<'_>,
    data: Py<PyBytes>,
    points_per_edge: usize,
    interior: bool,
    parallel: Option<bool>,
) -> PyResult<Py<PyBytes>> {
    let data = data.as_bytes(py);
    let count = checked_chunks(data, 8, "cell")?;
    let parallel = use_parallel(parallel, count, BOUNDARY_PARALLEL_THRESHOLD);
    let output = py
        .detach(|| {
            let cells = decode_cells(data)?;
            let boundaries = RhealpixDggs::wgs84_003().boundaries_lonlat_bulk(
                &cells,
                points_per_edge,
                interior,
                parallel,
            )?;
            let point_count = points_per_edge
                .checked_sub(1)
                .and_then(|value| value.checked_mul(4))
                .unwrap_or(0);
            let mut output = Vec::with_capacity(boundaries.len() * point_count * 16);
            for boundary in boundaries {
                for (longitude, latitude) in boundary {
                    output.extend_from_slice(&latitude.to_le_bytes());
                    output.extend_from_slice(&longitude.to_le_bytes());
                }
            }
            Ok::<_, Error>(output)
        })
        .map_err(value_error)?;
    Ok(PyBytes::new(py, &output).unbind())
}

/// Rust buffer backend for ragged bounding-box coverage batches.
#[pyfunction(name = "_bboxes_to_cells_buffer", signature = (data, resolution, parallel=None))]
fn bboxes_to_cells_buffer(
    py: Python<'_>,
    data: Py<PyBytes>,
    resolution: u8,
    parallel: Option<bool>,
) -> PyResult<(Py<PyBytes>, Py<PyBytes>)> {
    let data = data.as_bytes(py);
    let count = checked_chunks(data, 32, "bounding-box")?;
    let parallel = use_parallel(parallel, count, REGION_PARALLEL_THRESHOLD);
    let (cell_bytes, offset_bytes) = py
        .detach(|| {
            let bboxes: Vec<_> = data
                .chunks_exact(32)
                .map(|chunk| {
                    (
                        f64_at(chunk, 0),
                        f64_at(chunk, 8),
                        f64_at(chunk, 16),
                        f64_at(chunk, 24),
                    )
                })
                .collect();
            let groups =
                RhealpixDggs::wgs84_003().cells_from_bboxes_bulk(&bboxes, resolution, parallel)?;
            let total: usize = groups.iter().map(Vec::len).sum();
            let mut cell_bytes = Vec::with_capacity(total * 8);
            let mut offset_bytes = Vec::with_capacity((groups.len() + 1) * 8);
            let mut offset = 0_u64;
            offset_bytes.extend_from_slice(&offset.to_le_bytes());
            for cells in groups {
                cell_bytes.extend_from_slice(&encode_cells(&cells));
                let cell_count =
                    u64::try_from(cells.len()).map_err(|_| Error::ExpansionTooLarge(u64::MAX))?;
                offset = offset
                    .checked_add(cell_count)
                    .ok_or(Error::ExpansionTooLarge(u64::MAX))?;
                offset_bytes.extend_from_slice(&offset.to_le_bytes());
            }
            Ok::<_, Error>((cell_bytes, offset_bytes))
        })
        .map_err(value_error)?;
    Ok((
        PyBytes::new(py, &cell_bytes).unbind(),
        PyBytes::new(py, &offset_bytes).unbind(),
    ))
}

/// Return a cell nucleus as `(latitude, longitude)` degrees.
#[pyfunction]
fn cell_to_latlng(cell: &str) -> PyResult<(f64, f64)> {
    let cell = parse_cell(cell)?;
    RhealpixDggs::wgs84_003()
        .cell_to_lonlat(&cell)
        .map(|(longitude, latitude)| (latitude, longitude))
        .map_err(value_error)
}

/// Return an ellipsoidal cell centroid as `(latitude, longitude)` degrees.
#[pyfunction]
fn cell_to_centroid(cell: &str) -> PyResult<(f64, f64)> {
    let cell = parse_cell(cell)?;
    RhealpixDggs::wgs84_003()
        .cell_centroid_lonlat(&cell)
        .map(|(longitude, latitude)| (latitude, longitude))
        .map_err(value_error)
}

/// Return cells covering a latitude/longitude bounding box.
///
/// Antimeridian-crossing boxes use `west > east` and are split automatically.
#[pyfunction]
fn bbox_to_cells(
    north: f64,
    south: f64,
    east: f64,
    west: f64,
    resolution: u8,
) -> PyResult<Vec<String>> {
    let dggs = RhealpixDggs::wgs84_003();
    let intervals = if west <= east {
        vec![(west, east)]
    } else {
        vec![(west, 180.0), (-180.0, east)]
    };
    let mut cells = std::collections::BTreeSet::new();
    for (interval_west, interval_east) in intervals {
        for cell in dggs
            .cells_from_region_lonlat(resolution, (interval_west, north), (interval_east, south))
            .map_err(value_error)?
            .into_iter()
            .flatten()
        {
            cells.insert(cell);
        }
    }
    Ok(cells.into_iter().map(|cell| cell.to_string()).collect())
}

/// Return cells touched by a latitude/longitude polyline in path order.
#[pyfunction]
fn line_to_cells(coordinates: Vec<(f64, f64)>, resolution: u8) -> PyResult<Vec<String>> {
    let coordinates: Vec<_> = coordinates
        .into_iter()
        .map(|(latitude, longitude)| (longitude, latitude))
        .collect();
    RhealpixDggs::wgs84_003()
        .cells_from_polyline_lonlat(resolution, &coordinates)
        .map(|cells| cells.into_iter().map(|cell| cell.to_string()).collect())
        .map_err(value_error)
}

/// Fill a latitude/longitude polygon using cell-centroid containment.
#[pyfunction(signature = (exterior, resolution, holes=None, compact=false))]
fn polygon_to_cells(
    exterior: Vec<(f64, f64)>,
    resolution: u8,
    holes: Option<Vec<Vec<(f64, f64)>>>,
    compact: bool,
) -> PyResult<Vec<String>> {
    let exterior: Vec<_> = exterior
        .into_iter()
        .map(|(latitude, longitude)| (longitude, latitude))
        .collect();
    let holes: Vec<Vec<_>> = holes
        .unwrap_or_default()
        .into_iter()
        .map(|ring| {
            ring.into_iter()
                .map(|(latitude, longitude)| (longitude, latitude))
                .collect()
        })
        .collect();
    RhealpixDggs::wgs84_003()
        .cells_from_polygon_lonlat(resolution, &exterior, &holes, compact)
        .map(|cells| cells.into_iter().map(|cell| cell.to_string()).collect())
        .map_err(value_error)
}

/// Return boundary points as `(latitude, longitude)` degrees.
#[pyfunction(signature = (cell, trim_dart=false))]
fn cell_to_boundary(cell: &str, trim_dart: bool) -> PyResult<Vec<(f64, f64)>> {
    let cell = parse_cell(cell)?;
    RhealpixDggs::wgs84_003()
        .cell_vertices_lonlat(&cell, trim_dart)
        .map(|points| {
            points
                .into_iter()
                .map(|(longitude, latitude)| (latitude, longitude))
                .collect()
        })
        .map_err(value_error)
}

/// Return exactly `4 * points_per_edge - 4` boundary points as
/// `(latitude, longitude)` degrees.
#[pyfunction(signature = (cell, points_per_edge=2, interior=false))]
fn cell_to_boundary_densified(
    cell: &str,
    points_per_edge: usize,
    interior: bool,
) -> PyResult<Vec<(f64, f64)>> {
    let cell = parse_cell(cell)?;
    RhealpixDggs::wgs84_003()
        .cell_boundary_lonlat(&cell, points_per_edge, interior)
        .map(|points| {
            points
                .into_iter()
                .map(|(longitude, latitude)| (latitude, longitude))
                .collect()
        })
        .map_err(value_error)
}

/// Return ordered, shared-edge-deduplicated cell boundaries.
#[pyfunction(signature = (cells, points_per_edge=2, interior=false, parallel=None))]
fn cells_to_boundaries(
    py: Python<'_>,
    cells: Vec<String>,
    points_per_edge: usize,
    interior: bool,
    parallel: Option<bool>,
) -> PyResult<Vec<Vec<(f64, f64)>>> {
    let cells = cells
        .iter()
        .map(String::as_str)
        .map(parse_cell)
        .collect::<PyResult<Vec<_>>>()?;
    let use_parallel = use_parallel(parallel, cells.len(), BOUNDARY_PARALLEL_THRESHOLD);
    py.detach(move || {
        RhealpixDggs::wgs84_003().boundaries_lonlat_bulk(
            &cells,
            points_per_edge,
            interior,
            use_parallel,
        )
    })
    .map(|boundaries| {
        boundaries
            .into_iter()
            .map(|boundary| {
                boundary
                    .into_iter()
                    .map(|(longitude, latitude)| (latitude, longitude))
                    .collect()
            })
            .collect()
    })
    .map_err(value_error)
}

/// Return the upstream geographic region name for a cell.
#[pyfunction]
fn get_cell_region(cell: &str) -> PyResult<&'static str> {
    Ok(parse_cell(cell)?.region().as_str())
}

/// Return the upstream ellipsoidal shape name for a cell.
#[pyfunction]
fn get_cell_shape(cell: &str) -> PyResult<&'static str> {
    Ok(parse_cell(cell)?.shape().as_str())
}

/// Return one WGS84_003 planar or ellipsoidal edge neighbour.
#[pyfunction(signature = (cell, direction, plane=true))]
fn cell_to_neighbor(cell: &str, direction: &str, plane: bool) -> PyResult<Option<String>> {
    let cell = parse_cell(cell)?;
    let dggs = RhealpixDggs::wgs84_003();
    if plane {
        let direction = Direction::from_str(direction).map_err(value_error)?;
        Ok(Some(dggs.planar_neighbor(&cell, direction).to_string()))
    } else {
        let direction = EllipsoidalDirection::from_str(direction).map_err(value_error)?;
        dggs.ellipsoidal_neighbor(&cell, direction)
            .map(|neighbour| neighbour.map(|value| value.to_string()))
            .map_err(value_error)
    }
}

/// Return all four WGS84_003 planar or ellipsoidal edge neighbours.
#[pyfunction(signature = (cell, plane=true))]
fn cell_to_neighbors(cell: &str, plane: bool) -> PyResult<BTreeMap<String, String>> {
    let cell = parse_cell(cell)?;
    let dggs = RhealpixDggs::wgs84_003();
    configured_neighbors(&dggs, &cell, plane)
}

/// Return whether two same-resolution WGS84_003 cells share an edge.
#[pyfunction]
fn are_neighbor_cells(origin: &str, destination: &str) -> PyResult<bool> {
    let origin = parse_cell(origin)?;
    let destination = parse_cell(destination)?;
    RhealpixDggs::wgs84_003()
        .are_neighbor_cells(&origin, &destination)
        .map_err(value_error)
}

fn configured_cell_relation(
    dggs: &RhealpixDggs,
    left: &str,
    right: &str,
    relation: &str,
) -> PyResult<bool> {
    let left = parse_cell(left)?;
    let right = parse_cell(right)?;
    match relation {
        "equals" => Ok(left.equals(&right)),
        "within" => Ok(left.within(&right)),
        "contains" => Ok(left.contains(&right)),
        "covers" => Ok(left.covers(&right)),
        "covered_by" => Ok(left.covered_by(&right)),
        "touches" => dggs.cells_touch(&left, &right).map_err(value_error),
        "disjoint" => dggs.cells_are_disjoint(&left, &right).map_err(value_error),
        "intersects" => dggs.cells_intersect(&left, &right).map_err(value_error),
        "crosses" => Ok(RhealpixDggs::cells_cross(&left, &right)),
        "overlaps" => Ok(RhealpixDggs::cells_topologically_overlap(&left, &right)),
        _ => Err(PyValueError::new_err("unknown cell relation")),
    }
}

macro_rules! cell_relation_function {
    ($name:ident, $relation:literal, $doc:literal) => {
        #[doc = $doc]
        #[pyfunction]
        fn $name(left: &str, right: &str) -> PyResult<bool> {
            configured_cell_relation(&RhealpixDggs::wgs84_003(), left, right, $relation)
        }
    };
}

cell_relation_function!(
    cell_equals,
    "equals",
    "Return whether two WGS84_003 cells are equal."
);
cell_relation_function!(
    cell_within,
    "within",
    "Return whether the first cell is within the second."
);
cell_relation_function!(
    cell_contains,
    "contains",
    "Return whether the first cell contains the second."
);
cell_relation_function!(
    cell_covers,
    "covers",
    "Return whether the first cell covers the second."
);
cell_relation_function!(
    cell_covered_by,
    "covered_by",
    "Return whether the first cell is covered by the second."
);
cell_relation_function!(
    cell_touches,
    "touches",
    "Return whether two cell boundaries touch without interior overlap."
);
cell_relation_function!(
    cell_disjoint,
    "disjoint",
    "Return whether two cells share no point."
);
cell_relation_function!(
    cell_intersects,
    "intersects",
    "Return whether two closed cells share any point."
);
cell_relation_function!(
    cell_crosses,
    "crosses",
    "Return the OGC crosses predicate for two cells."
);
cell_relation_function!(
    cell_overlaps,
    "overlaps",
    "Return the OGC overlaps predicate for two cells."
);

/// Compatibility-facade cell relation with configurable polar squares.
#[pyfunction(name = "_cell_relation")]
fn compat_cell_relation(
    left: &str,
    right: &str,
    relation: &str,
    north_square: u8,
    south_square: u8,
) -> PyResult<bool> {
    configured_cell_relation(
        &configured_dggs(north_square, south_square),
        left,
        right,
        relation,
    )
}

/// Return WGS84_003 cells within `k` edge-neighbour steps.
#[pyfunction]
fn grid_disk(py: Python<'_>, origin: &str, k: i64) -> PyResult<Vec<String>> {
    let origin = parse_cell(origin)?;
    let k = parse_grid_distance(k)?;
    py.detach(move || RhealpixDggs::wgs84_003().grid_disk(&origin, k))
        .map(|cells| cells.into_iter().map(|cell| cell.to_string()).collect())
        .map_err(value_error)
}

/// Return WGS84_003 cells exactly `k` edge-neighbour steps away.
#[pyfunction]
fn grid_ring(py: Python<'_>, origin: &str, k: i64) -> PyResult<Vec<String>> {
    let origin = parse_cell(origin)?;
    let k = parse_grid_distance(k)?;
    py.detach(move || RhealpixDggs::wgs84_003().grid_ring(&origin, k))
        .map(|cells| cells.into_iter().map(|cell| cell.to_string()).collect())
        .map_err(value_error)
}

fn configured_dggs(north_square: u8, south_square: u8) -> RhealpixDggs {
    RhealpixDggs::new(Ellipsoid::wgs84(), north_square, south_square)
}

fn region_hint(region: &str) -> PyResult<Option<Region>> {
    match region {
        "none" => Ok(None),
        "north_polar" => Ok(Some(Region::NorthPolar)),
        "equatorial" => Ok(Some(Region::Equatorial)),
        "south_polar" => Ok(Some(Region::SouthPolar)),
        _ => Err(PyValueError::new_err(
            "region must be 'none', 'north_polar', 'equatorial', or 'south_polar'",
        )),
    }
}

/// Compatibility-facade HEALPix and rHEALPix projection.
#[pyfunction(name = "_project", signature = (point, projection="rhealpix", inverse=false, region="none", north_square=0, south_square=0))]
fn compat_project(
    point: (f64, f64),
    projection: &str,
    inverse: bool,
    region: &str,
    north_square: u8,
    south_square: u8,
) -> PyResult<(f64, f64)> {
    let dggs = configured_dggs(north_square, south_square);
    let result = match (projection, inverse) {
        ("rhealpix", false) => dggs.project_lonlat(point.0, point.1),
        ("rhealpix", true) => match region_hint(region)? {
            None => dggs.unproject_lonlat(point.0, point.1),
            Some(region) => dggs.unproject_lonlat_in_region(point.0, point.1, region),
        },
        ("healpix", false) => dggs.project_healpix_lonlat(point.0, point.1),
        ("healpix", true) => dggs.unproject_healpix_lonlat(point.0, point.1),
        _ => {
            return Err(PyValueError::new_err(
                "projection must be 'healpix' or 'rhealpix'",
            ));
        }
    };
    result.map_err(value_error)
}

/// Compatibility-facade HEALPix triangle rearrangement.
#[pyfunction(name = "_combine_triangles", signature = (point, inverse=false, region="none", north_square=0, south_square=0))]
fn compat_combine_triangles(
    point: (f64, f64),
    inverse: bool,
    region: &str,
    north_square: u8,
    south_square: u8,
) -> PyResult<(f64, f64)> {
    configured_dggs(north_square, south_square)
        .combine_triangles(point.0, point.1, inverse, region_hint(region)?)
        .map_err(value_error)
}

/// Compatibility-facade HEALPix triangle classifier.
#[pyfunction(name = "_triangle", signature = (point, inverse=true, north_square=0, south_square=0))]
fn compat_triangle(
    point: (f64, f64),
    inverse: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<(Option<u8>, &'static str)> {
    configured_dggs(north_square, south_square)
        .triangle(point.0, point.1, inverse)
        .map(|(number, region)| (number, region.as_str()))
        .map_err(value_error)
}

/// Compatibility-facade ellipsoidal Cartesian coordinates.
#[pyfunction(name = "_xyz", signature = (point, lonlat=false, north_square=0, south_square=0))]
fn compat_xyz(
    point: (f64, f64),
    lonlat: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<(f64, f64, f64)> {
    let dggs = configured_dggs(north_square, south_square);
    if lonlat {
        dggs.xyz_lonlat(point.0, point.1).map_err(value_error)
    } else {
        dggs.xyz_projected(point.0, point.1).map_err(value_error)
    }
}

/// Compatibility-facade folded cube coordinates.
#[pyfunction(name = "_xyz_cube", signature = (point, lonlat=false, north_square=0, south_square=0))]
fn compat_xyz_cube(
    point: (f64, f64),
    lonlat: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<(f64, f64, f64)> {
    let dggs = configured_dggs(north_square, south_square);
    if lonlat {
        dggs.xyz_cube_lonlat(point.0, point.1).map_err(value_error)
    } else {
        dggs.xyz_cube_projected(point.0, point.1)
            .map_err(value_error)
    }
}

fn configured_neighbors(
    dggs: &RhealpixDggs,
    cell: &CellId,
    plane: bool,
) -> PyResult<BTreeMap<String, String>> {
    Ok(configured_neighbor_pairs(dggs, cell, plane)?
        .into_iter()
        .collect())
}

fn configured_neighbor_pairs(
    dggs: &RhealpixDggs,
    cell: &CellId,
    plane: bool,
) -> PyResult<Vec<(String, String)>> {
    if plane {
        Ok(Direction::ALL
            .into_iter()
            .map(|direction| {
                (
                    direction.as_str().to_owned(),
                    dggs.planar_neighbor(cell, direction).to_string(),
                )
            })
            .collect())
    } else {
        dggs.ellipsoidal_neighbors(cell)
            .map(|neighbours| {
                neighbours
                    .into_iter()
                    .map(|(direction, neighbour)| (direction.to_string(), neighbour.to_string()))
                    .collect()
            })
            .map_err(value_error)
    }
}

/// Compatibility-facade point indexing with upstream coordinate ordering.
#[pyfunction(name = "_cell_from_point", signature = (resolution, point, plane=true, north_square=0, south_square=0))]
fn compat_cell_from_point(
    resolution: u8,
    point: (f64, f64),
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<Option<String>> {
    let dggs = configured_dggs(north_square, south_square);
    let result = if plane {
        dggs.cell_from_projected(point.0, point.1, resolution)
    } else {
        dggs.cell_from_lonlat(point.0, point.1, resolution)
    };
    match result {
        Ok(cell) => Ok(Some(cell.to_string())),
        Err(Error::OutsideProjection) => Ok(None),
        Err(error) => Err(value_error(error)),
    }
}

/// Compatibility-facade nucleus with upstream coordinate ordering.
#[pyfunction(name = "_cell_nucleus", signature = (cell, plane=true, north_square=0, south_square=0))]
fn compat_cell_nucleus(
    cell: &str,
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<(f64, f64)> {
    let cell = parse_cell(cell)?;
    let dggs = configured_dggs(north_square, south_square);
    if plane {
        dggs.cell_to_projected(&cell).map_err(value_error)
    } else {
        dggs.cell_to_lonlat(&cell).map_err(value_error)
    }
}

/// Compatibility-facade centroid with upstream coordinate ordering.
#[pyfunction(name = "_cell_centroid", signature = (cell, plane=true, north_square=0, south_square=0))]
fn compat_cell_centroid(
    cell: &str,
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<(f64, f64)> {
    let cell = parse_cell(cell)?;
    let dggs = configured_dggs(north_square, south_square);
    if plane {
        dggs.cell_to_projected(&cell).map_err(value_error)
    } else {
        dggs.cell_centroid_lonlat(&cell).map_err(value_error)
    }
}

/// Compatibility-facade rectangle coverage with upstream coordinate ordering.
#[pyfunction(name = "_cells_from_region", signature = (resolution, upper_left, lower_right, plane=true, north_square=0, south_square=0))]
fn compat_cells_from_region(
    resolution: u8,
    upper_left: (f64, f64),
    lower_right: (f64, f64),
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<Vec<Vec<String>>> {
    let dggs = configured_dggs(north_square, south_square);
    let rows = if plane {
        dggs.cells_from_region_projected(resolution, upper_left, lower_right)
    } else {
        dggs.cells_from_region_lonlat(resolution, upper_left, lower_right)
    }
    .map_err(value_error)?;
    Ok(rows
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.to_string()).collect())
        .collect())
}

/// Compatibility-facade two-point line coverage with upstream ordering.
#[pyfunction(name = "_cells_from_line", signature = (resolution, start, end, plane=true, north_square=0, south_square=0))]
fn compat_cells_from_line(
    resolution: u8,
    start: (f64, f64),
    end: (f64, f64),
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<Vec<String>> {
    let dggs = configured_dggs(north_square, south_square);
    let cells = if plane {
        dggs.cells_from_polyline_projected(resolution, &[start, end])
    } else {
        dggs.cells_from_polyline_lonlat(resolution, &[start, end])
    }
    .map_err(value_error)?;
    Ok(cells.into_iter().map(|cell| cell.to_string()).collect())
}

/// Compatibility-facade vertices with upstream coordinate ordering.
#[pyfunction(name = "_cell_vertices", signature = (cell, plane=true, trim_dart=false, north_square=0, south_square=0))]
fn compat_cell_vertices(
    cell: &str,
    plane: bool,
    trim_dart: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<Vec<(f64, f64)>> {
    let cell = parse_cell(cell)?;
    let dggs = configured_dggs(north_square, south_square);
    if plane {
        dggs.cell_vertices_projected(&cell)
            .map(|points| points.into_iter().collect())
            .map_err(value_error)
    } else {
        dggs.cell_vertices_lonlat(&cell, trim_dart)
            .map_err(value_error)
    }
}

/// Compatibility-facade upper-left or geographic northwest vertex.
#[pyfunction(name = "_cell_vertex", signature = (cell, vertex="upper_left", plane=true, north_square=0, south_square=0))]
fn compat_cell_vertex(
    cell: &str,
    vertex: &str,
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<(f64, f64)> {
    let cell = parse_cell(cell)?;
    let dggs = configured_dggs(north_square, south_square);
    let point = match (vertex, plane) {
        ("upper_left", true) => dggs.cell_upper_left_projected(&cell),
        ("upper_left", false) => dggs.cell_upper_left_lonlat(&cell),
        ("northwest", true) => dggs.cell_northwest_vertex_projected(&cell),
        ("northwest", false) => dggs.cell_northwest_vertex_lonlat(&cell),
        _ => {
            return Err(PyValueError::new_err(
                "vertex must be 'upper_left' or 'northwest'",
            ));
        }
    };
    point.map_err(value_error)
}

/// Compatibility-facade boundary with upstream coordinate ordering.
#[pyfunction(name = "_cell_boundary", signature = (cell, n=2, plane=true, interior=false, north_square=0, south_square=0))]
fn compat_cell_boundary(
    cell: &str,
    n: usize,
    plane: bool,
    interior: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<Vec<(f64, f64)>> {
    let cell = parse_cell(cell)?;
    let dggs = configured_dggs(north_square, south_square);
    if plane {
        dggs.cell_boundary_projected(&cell, n, interior)
            .map_err(value_error)
    } else {
        dggs.cell_boundary_lonlat_compatible(&cell, n, interior)
            .map_err(value_error)
    }
}

/// Compatibility-facade neighbour with configurable polar squares.
#[pyfunction(name = "_cell_neighbor", signature = (cell, direction, plane=true, north_square=0, south_square=0))]
fn compat_cell_neighbor(
    cell: &str,
    direction: &str,
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<Option<String>> {
    let cell = parse_cell(cell)?;
    let dggs = configured_dggs(north_square, south_square);
    if plane {
        let direction = match Direction::from_str(direction) {
            Ok(direction) => direction,
            Err(_) => return Ok(None),
        };
        Ok(Some(dggs.planar_neighbor(&cell, direction).to_string()))
    } else {
        let direction = match EllipsoidalDirection::from_str(direction) {
            Ok(direction) => direction,
            Err(_) => return Ok(None),
        };
        dggs.ellipsoidal_neighbor(&cell, direction)
            .map(|neighbour| neighbour.map(|value| value.to_string()))
            .map_err(value_error)
    }
}

/// Compatibility-facade neighbours with configurable polar squares.
#[pyfunction(name = "_cell_neighbors", signature = (cell, plane=true, north_square=0, south_square=0))]
fn compat_cell_neighbors(
    cell: &str,
    plane: bool,
    north_square: u8,
    south_square: u8,
) -> PyResult<Vec<(String, String)>> {
    let cell = parse_cell(cell)?;
    configured_neighbor_pairs(&configured_dggs(north_square, south_square), &cell, plane)
}

/// Compatibility-facade cell width or area.
#[pyfunction(name = "_cell_metric", signature = (resolution, metric, plane=true))]
fn compat_cell_metric(resolution: u8, metric: &str, plane: bool) -> PyResult<Option<f64>> {
    let dggs = RhealpixDggs::wgs84_003();
    match metric {
        "width" => {
            if plane {
                dggs.cell_width(resolution).map(Some).map_err(value_error)
            } else {
                Ok(None)
            }
        }
        "area" => {
            if plane {
                dggs.cell_width(resolution)
                    .map(|width| Some(width * width))
                    .map_err(value_error)
            } else {
                dggs.cell_area(resolution).map(Some).map_err(value_error)
            }
        }
        _ => Err(PyValueError::new_err("metric must be 'width' or 'area'")),
    }
}

/// Return the direct parent or the ancestor at `resolution`.
#[pyfunction(signature = (cell, resolution=None))]
fn cell_to_parent(cell: &str, resolution: Option<u8>) -> PyResult<Option<String>> {
    let cell = parse_cell(cell)?;
    let parent = if let Some(resolution) = resolution {
        Some(cell.parent_at(resolution).map_err(value_error)?)
    } else {
        cell.parent()
    };
    Ok(parent.map(|value| value.to_string()))
}

/// Return direct children or all descendants at `resolution`.
#[pyfunction(signature = (cell, resolution=None))]
fn cell_to_children(cell: &str, resolution: Option<u8>) -> PyResult<Vec<String>> {
    let cell = parse_cell(cell)?;
    let resolution = resolution.unwrap_or_else(|| cell.resolution().saturating_add(1));
    cell.descendants(resolution)
        .map(|cells| cells.into_iter().map(|value| value.to_string()).collect())
        .map_err(value_error)
}

/// Return the next cell at the requested resolution in post-order traversal.
#[pyfunction(signature = (cell, resolution=None))]
fn cell_to_successor(cell: &str, resolution: Option<u8>) -> PyResult<Option<String>> {
    let cell = parse_cell(cell)?;
    let successor = match resolution {
        Some(resolution) => cell.successor_at(resolution).map_err(value_error)?,
        None => cell.successor(),
    };
    Ok(successor.map(|value| value.to_string()))
}

/// Return the previous cell at the requested resolution in post-order traversal.
#[pyfunction(signature = (cell, resolution=None))]
fn cell_to_predecessor(cell: &str, resolution: Option<u8>) -> PyResult<Option<String>> {
    let cell = parse_cell(cell)?;
    let predecessor = match resolution {
        Some(resolution) => cell.predecessor_at(resolution).map_err(value_error)?,
        None => cell.predecessor(),
    };
    Ok(predecessor.map(|value| value.to_string()))
}

/// Return the stable zero-based level-order index.
#[pyfunction]
fn cell_to_level_order_index(cell: &str) -> PyResult<u64> {
    Ok(parse_cell(cell)?.level_order_index())
}

/// Construct a cell from its stable zero-based level-order index.
#[pyfunction]
fn level_order_index_to_cell(index: u64) -> PyResult<String> {
    CellId::from_level_order_index(index)
        .map(|cell| cell.to_string())
        .map_err(value_error)
}

/// Return the zero-based post-order index in the finite hierarchy.
#[pyfunction]
fn cell_to_post_order_index(cell: &str) -> PyResult<u64> {
    Ok(parse_cell(cell)?.post_order_index())
}

/// Construct a cell from its zero-based post-order index.
#[pyfunction]
fn post_order_index_to_cell(index: u64) -> PyResult<String> {
    CellId::from_post_order_index(index)
        .map(|cell| cell.to_string())
        .map_err(value_error)
}

/// Compare two cell IDs using upstream post-order traversal.
#[pyfunction(name = "_compare_cells")]
fn compat_compare_cells(left: &str, right: &str) -> PyResult<i8> {
    let left = parse_cell(left)?;
    let right = parse_cell(right)?;
    Ok(match left.cmp(&right) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    })
}

/// Return the cell resolution.
#[pyfunction]
fn get_resolution(cell: &str) -> PyResult<u8> {
    Ok(parse_cell(cell)?.resolution())
}

/// Return the zero-based resolution-zero face number.
#[pyfunction]
fn get_base_cell_number(cell: &str) -> PyResult<u8> {
    Ok(parse_cell(cell)?.face().number())
}

/// Return whether a string is a canonical cell ID.
#[pyfunction]
fn is_valid_cell(cell: &str) -> bool {
    CellId::from_str(cell).is_ok()
}

/// Convert a string cell ID to its stable integer representation.
#[pyfunction]
fn str_to_int(cell: &str) -> PyResult<u64> {
    Ok(parse_cell(cell)?.to_u64())
}

/// Convert a stable integer representation to a string cell ID.
#[pyfunction]
fn int_to_str(cell: u64) -> PyResult<String> {
    CellId::from_u64(cell)
        .map(|value| value.to_string())
        .map_err(value_error)
}

/// Return equal-area cell area in square metres or square kilometres.
#[pyfunction(signature = (cell, unit="m^2"))]
fn cell_area(cell: &str, unit: &str) -> PyResult<f64> {
    let resolution = parse_cell(cell)?.resolution();
    let area = RhealpixDggs::wgs84_003()
        .cell_area(resolution)
        .map_err(value_error)?;
    match unit {
        "m^2" | "m2" => Ok(area),
        "km^2" | "km2" => Ok(area / 1_000_000.0),
        _ => Err(PyValueError::new_err(
            "unit must be one of: 'm^2', 'm2', 'km^2', 'km2'",
        )),
    }
}

/// Recursively compact complete groups of nine sibling cells.
#[pyfunction]
fn compact_cells(cells: Vec<String>) -> PyResult<Vec<String>> {
    let cells = cells
        .iter()
        .map(String::as_str)
        .map(parse_cell)
        .collect::<PyResult<Vec<_>>>()?;
    Ok(compact_core(cells)
        .into_iter()
        .map(|cell| cell.to_string())
        .collect())
}

/// Expand cells to a common resolution.
#[pyfunction]
fn uncompact_cells(cells: Vec<String>, resolution: u8) -> PyResult<Vec<String>> {
    let cells = cells
        .iter()
        .map(String::as_str)
        .map(parse_cell)
        .collect::<PyResult<Vec<_>>>()?;
    uncompact_core(cells, resolution)
        .map(|values| values.into_iter().map(|cell| cell.to_string()).collect())
        .map_err(value_error)
}

/// Rust-backed rHEALPix functions with an H3-like Python surface.
#[pymodule]
fn _rhealpixdggs(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(latlng_to_cell, module)?)?;
    module.add_function(wrap_pyfunction!(latlngs_to_cells, module)?)?;
    module.add_function(wrap_pyfunction!(latlngs_to_cells_buffer, module)?)?;
    module.add_function(wrap_pyfunction!(cells_to_latlngs_buffer, module)?)?;
    module.add_function(wrap_pyfunction!(cells_to_boundaries_buffer, module)?)?;
    module.add_function(wrap_pyfunction!(bboxes_to_cells_buffer, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_latlng, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_centroid, module)?)?;
    module.add_function(wrap_pyfunction!(bbox_to_cells, module)?)?;
    module.add_function(wrap_pyfunction!(line_to_cells, module)?)?;
    module.add_function(wrap_pyfunction!(polygon_to_cells, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_boundary, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_boundary_densified, module)?)?;
    module.add_function(wrap_pyfunction!(cells_to_boundaries, module)?)?;
    module.add_function(wrap_pyfunction!(get_cell_region, module)?)?;
    module.add_function(wrap_pyfunction!(get_cell_shape, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_neighbor, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_neighbors, module)?)?;
    module.add_function(wrap_pyfunction!(are_neighbor_cells, module)?)?;
    module.add_function(wrap_pyfunction!(cell_equals, module)?)?;
    module.add_function(wrap_pyfunction!(cell_within, module)?)?;
    module.add_function(wrap_pyfunction!(cell_contains, module)?)?;
    module.add_function(wrap_pyfunction!(cell_covers, module)?)?;
    module.add_function(wrap_pyfunction!(cell_covered_by, module)?)?;
    module.add_function(wrap_pyfunction!(cell_touches, module)?)?;
    module.add_function(wrap_pyfunction!(cell_disjoint, module)?)?;
    module.add_function(wrap_pyfunction!(cell_intersects, module)?)?;
    module.add_function(wrap_pyfunction!(cell_crosses, module)?)?;
    module.add_function(wrap_pyfunction!(cell_overlaps, module)?)?;
    module.add_function(wrap_pyfunction!(grid_disk, module)?)?;
    module.add_function(wrap_pyfunction!(grid_ring, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_parent, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_children, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_successor, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_predecessor, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_level_order_index, module)?)?;
    module.add_function(wrap_pyfunction!(level_order_index_to_cell, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_post_order_index, module)?)?;
    module.add_function(wrap_pyfunction!(post_order_index_to_cell, module)?)?;
    module.add_function(wrap_pyfunction!(get_resolution, module)?)?;
    module.add_function(wrap_pyfunction!(get_base_cell_number, module)?)?;
    module.add_function(wrap_pyfunction!(is_valid_cell, module)?)?;
    module.add_function(wrap_pyfunction!(str_to_int, module)?)?;
    module.add_function(wrap_pyfunction!(int_to_str, module)?)?;
    module.add_function(wrap_pyfunction!(cell_area, module)?)?;
    module.add_function(wrap_pyfunction!(compact_cells, module)?)?;
    module.add_function(wrap_pyfunction!(uncompact_cells, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_from_point, module)?)?;
    module.add_function(wrap_pyfunction!(compat_project, module)?)?;
    module.add_function(wrap_pyfunction!(compat_combine_triangles, module)?)?;
    module.add_function(wrap_pyfunction!(compat_triangle, module)?)?;
    module.add_function(wrap_pyfunction!(compat_xyz, module)?)?;
    module.add_function(wrap_pyfunction!(compat_xyz_cube, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_nucleus, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_centroid, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cells_from_region, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cells_from_line, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_vertex, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_boundary, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_neighbor, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_neighbors, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_metric, module)?)?;
    module.add_function(wrap_pyfunction!(compat_compare_cells, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_relation, module)?)?;
    module.add("MAX_RESOLUTION", MAX_RESOLUTION)?;
    module.add("POINT_PARALLEL_THRESHOLD", POINT_PARALLEL_THRESHOLD)?;
    module.add("BOUNDARY_PARALLEL_THRESHOLD", BOUNDARY_PARALLEL_THRESHOLD)?;
    module.add("REGION_PARALLEL_THRESHOLD", REGION_PARALLEL_THRESHOLD)?;
    module.add("PARALLELISM_AVAILABLE", parallelism_available())?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
