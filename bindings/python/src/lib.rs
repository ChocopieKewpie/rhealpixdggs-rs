//! PyO3 bindings for the dependency-free `rhealpixdggs` core.

use std::collections::BTreeMap;
use std::str::FromStr;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rhealpixdggs::{
    CellId, Direction, Ellipsoid, Error, MAX_RESOLUTION, RhealpixDggs,
    compact_cells as compact_core, uncompact_cells as uncompact_core,
};

fn value_error(error: Error) -> PyErr {
    PyValueError::new_err(error.to_string())
}

fn parse_cell(value: &str) -> PyResult<CellId> {
    CellId::from_str(value).map_err(value_error)
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
fn latlngs_to_cells(coordinates: Vec<(f64, f64)>, resolution: u8) -> PyResult<Vec<String>> {
    let dggs = RhealpixDggs::wgs84_003();
    coordinates
        .into_iter()
        .map(|(latitude, longitude)| {
            dggs.cell_from_lonlat(longitude, latitude, resolution)
                .map(|cell| cell.to_string())
                .map_err(value_error)
        })
        .collect()
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

/// Return one WGS84_003 planar edge neighbour.
#[pyfunction]
fn cell_to_neighbor(cell: &str, direction: &str) -> PyResult<String> {
    let cell = parse_cell(cell)?;
    let direction = Direction::from_str(direction).map_err(value_error)?;
    Ok(RhealpixDggs::wgs84_003()
        .planar_neighbor(&cell, direction)
        .to_string())
}

/// Return all four WGS84_003 planar edge neighbours.
#[pyfunction]
fn cell_to_neighbors(cell: &str) -> PyResult<BTreeMap<&'static str, String>> {
    let cell = parse_cell(cell)?;
    let dggs = RhealpixDggs::wgs84_003();
    Ok(Direction::ALL
        .into_iter()
        .map(|direction| {
            (
                direction.as_str(),
                dggs.planar_neighbor(&cell, direction).to_string(),
            )
        })
        .collect())
}

fn configured_dggs(north_square: u8, south_square: u8) -> RhealpixDggs {
    RhealpixDggs::new(Ellipsoid::wgs84(), north_square, south_square)
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

/// Compatibility-facade neighbour with configurable polar squares.
#[pyfunction(name = "_cell_neighbor", signature = (cell, direction, north_square=0, south_square=0))]
fn compat_cell_neighbor(
    cell: &str,
    direction: &str,
    north_square: u8,
    south_square: u8,
) -> PyResult<Option<String>> {
    let cell = parse_cell(cell)?;
    let direction = match Direction::from_str(direction) {
        Ok(direction) => direction,
        Err(_) => return Ok(None),
    };
    Ok(Some(
        configured_dggs(north_square, south_square)
            .planar_neighbor(&cell, direction)
            .to_string(),
    ))
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
    module.add_function(wrap_pyfunction!(cell_to_latlng, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_boundary, module)?)?;
    module.add_function(wrap_pyfunction!(get_cell_region, module)?)?;
    module.add_function(wrap_pyfunction!(get_cell_shape, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_neighbor, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_neighbors, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_parent, module)?)?;
    module.add_function(wrap_pyfunction!(cell_to_children, module)?)?;
    module.add_function(wrap_pyfunction!(get_resolution, module)?)?;
    module.add_function(wrap_pyfunction!(get_base_cell_number, module)?)?;
    module.add_function(wrap_pyfunction!(is_valid_cell, module)?)?;
    module.add_function(wrap_pyfunction!(str_to_int, module)?)?;
    module.add_function(wrap_pyfunction!(int_to_str, module)?)?;
    module.add_function(wrap_pyfunction!(cell_area, module)?)?;
    module.add_function(wrap_pyfunction!(compact_cells, module)?)?;
    module.add_function(wrap_pyfunction!(uncompact_cells, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_from_point, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_nucleus, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_vertices, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_neighbor, module)?)?;
    module.add_function(wrap_pyfunction!(compat_cell_metric, module)?)?;
    module.add("MAX_RESOLUTION", MAX_RESOLUTION)?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
