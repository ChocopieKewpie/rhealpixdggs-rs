"""Optional vector I/O helpers for rHEALPix polygon coverage.

The Rust core intentionally has no geometry or file-format dependencies.  This
module is the Python adapter: it accepts Shapely polygonal geometries, converts
them to cells with the Rust implementation, and writes interoperable vector
outputs with GeoPandas/Pyogrio.
"""

from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path
from typing import Any, Literal

from . import numpy as _numpy
from ._rhealpixdggs import (
    cell_area as _cell_area,
    compact_cells as _compact_cells,
    get_resolution as _get_resolution,
    polygon_to_cells as _polygon_to_cells,
    polygon_to_cells_intersects as _polygon_to_cells_intersects,
    str_to_int as _str_to_int,
)


def _geometry_dependencies() -> tuple[Any, Any]:
    try:
        import geopandas as geopandas
        import shapely as shapely
    except ModuleNotFoundError as error:
        raise ImportError(
            "GeoPackage support requires the optional geospatial dependencies; "
            "install them with `pip install 'rhealpixdggs-rs[geo]'`"
        ) from error
    return geopandas, shapely


def _polygon_members(geometry: Any) -> list[Any]:
    if geometry is None or getattr(geometry, "is_empty", True):
        raise ValueError("geometry must be a non-empty Polygon or MultiPolygon")
    if not getattr(geometry, "is_valid", False):
        raise ValueError("geometry must be valid")
    geometry_type = getattr(geometry, "geom_type", None)
    if geometry_type == "Polygon":
        return [geometry]
    if geometry_type == "MultiPolygon":
        return list(geometry.geoms)
    raise TypeError("geometry must be a Polygon or MultiPolygon")


def _ring_latlng(coordinates: Iterable[Any]) -> list[tuple[float, float]]:
    lonlats = [(float(point[0]), float(point[1])) for point in coordinates]
    if len(lonlats) > 1 and lonlats[0] == lonlats[-1]:
        lonlats.pop()
    return [(latitude, longitude) for longitude, latitude in lonlats]


def geometry_to_cells(
    geometry: Any,
    resolution: int,
    *,
    compact: bool = False,
    coverage_mode: Literal["centroid", "intersects"] = "centroid",
) -> list[str]:
    """Cover a Shapely Polygon/MultiPolygon with WGS84 rHEALPix cells.

    ``coverage_mode="centroid"`` preserves ``rhealpixdggs-py`` polyfill
    semantics. ``coverage_mode="intersects"`` selects cells whose closed
    geometry touches the polygon, including edge and corner contact.
    Coordinates must be longitude/latitude degrees (EPSG:4326).
    """

    if coverage_mode not in {"centroid", "intersects"}:
        raise ValueError("coverage_mode must be 'centroid' or 'intersects'")
    cover = (
        _polygon_to_cells
        if coverage_mode == "centroid"
        else _polygon_to_cells_intersects
    )
    cells: set[str] = set()
    for polygon in _polygon_members(geometry):
        exterior = _ring_latlng(polygon.exterior.coords)
        holes = [_ring_latlng(ring.coords) for ring in polygon.interiors]
        cells.update(
            cover(
                exterior,
                resolution,
                holes=holes,
                compact=False,
            )
        )
    ordered = sorted(cells)
    return _compact_cells(ordered) if compact else ordered


def _polygon_parts(geometry: Any) -> list[Any]:
    if geometry.is_empty:
        return []
    if geometry.geom_type == "Polygon":
        return [geometry] if geometry.area > 0 else []
    if geometry.geom_type in {"MultiPolygon", "GeometryCollection"}:
        parts: list[Any] = []
        for member in geometry.geoms:
            parts.extend(_polygon_parts(member))
        return parts
    return []


def _cell_multipolygon(boundary: Any, shapely: Any) -> Any:
    from shapely import affinity

    lonlats = [
        (float(longitude), float(latitude)) for latitude, longitude in boundary
    ]
    longitudes = [point[0] for point in lonlats]

    if max(longitudes) - min(longitudes) <= 180.0:
        polygon = shapely.make_valid(shapely.Polygon(lonlats))
        parts = _polygon_parts(polygon)
    else:
        shifted = [
            (longitude + 360.0 if longitude < 0.0 else longitude, latitude)
            for longitude, latitude in lonlats
        ]
        polygon = shapely.make_valid(shapely.Polygon(shifted))
        western = polygon.intersection(shapely.box(0.0, -90.0, 180.0, 90.0))
        eastern = polygon.intersection(shapely.box(180.0, -90.0, 360.0, 90.0))
        parts = _polygon_parts(western)
        parts.extend(
            affinity.translate(part, xoff=-360.0)
            for part in _polygon_parts(eastern)
        )

    if not parts:
        raise ValueError("cell boundary could not be represented as a polygon")
    return shapely.MultiPolygon(parts)


def cells_to_geodataframe(
    cells: Iterable[str],
    *,
    points_per_edge: int = 4,
    parallel: bool | None = None,
) -> Any:
    """Build an EPSG:4326 GeoDataFrame containing rHEALPix cell polygons.

    Cell polygons are always stored as MultiPolygon geometries. Cells crossing
    the antimeridian are split at ±180 degrees so they display correctly in
    conventional GIS software.
    """

    geopandas, shapely = _geometry_dependencies()
    identifiers = list(cells)
    if points_per_edge < 2:
        raise ValueError("points_per_edge must be at least 2")

    if identifiers:
        import numpy as np

        integers = np.fromiter(
            (_str_to_int(identifier) for identifier in identifiers),
            dtype=np.uint64,
            count=len(identifiers),
        )
        boundaries = _numpy.cells_to_boundaries(
            integers,
            points_per_edge=points_per_edge,
            parallel=parallel,
        )
        geometries = [
            _cell_multipolygon(boundary, shapely) for boundary in boundaries
        ]
    else:
        geometries = []

    return geopandas.GeoDataFrame(
        {
            "cell_id": identifiers,
            "resolution": [_get_resolution(identifier) for identifier in identifiers],
            "area_m2": [_cell_area(identifier, "m^2") for identifier in identifiers],
        },
        geometry=geometries,
        crs="EPSG:4326",
    )


def polygon_to_geodataframe(
    geometry: Any,
    resolution: int,
    *,
    compact: bool = False,
    points_per_edge: int = 4,
    parallel: bool | None = None,
    coverage_mode: Literal["centroid", "intersects"] = "centroid",
) -> Any:
    """Cover an EPSG:4326 polygon and return its cell polygons."""

    cells = geometry_to_cells(
        geometry,
        resolution,
        compact=compact,
        coverage_mode=coverage_mode,
    )
    return cells_to_geodataframe(
        cells,
        points_per_edge=points_per_edge,
        parallel=parallel,
    )


def write_geopackage(
    frame: Any,
    output: str | Path,
    *,
    layer: str = "rhealpix_cells",
    overwrite: bool = False,
) -> Path:
    """Write a cell GeoDataFrame to a GeoPackage and return its path."""

    output_path = Path(output)
    if output_path.exists() and not overwrite:
        raise FileExistsError(
            f"output already exists: {output_path}; pass overwrite=True to replace it"
        )
    output_path.parent.mkdir(parents=True, exist_ok=True)
    frame.to_file(
        output_path,
        layer=layer,
        driver="GPKG",
        engine="pyogrio",
    )
    return output_path


def polygon_file_to_geopackage(
    input_path: str | Path,
    output_path: str | Path,
    resolution: int,
    *,
    input_layer: str | None = None,
    output_layer: str = "rhealpix_cells",
    compact: bool = False,
    points_per_edge: int = 4,
    parallel: bool | None = None,
    overwrite: bool = False,
    coverage_mode: Literal["centroid", "intersects"] = "centroid",
) -> Any:
    """Convert polygon features from a vector file into a cell GeoPackage.

    All polygon features are dissolved before coverage. The input must declare
    a CRS; it is reprojected to EPSG:4326 when necessary. The returned object is
    the GeoDataFrame that was written.
    """

    geopandas, _ = _geometry_dependencies()
    source = geopandas.read_file(input_path, layer=input_layer, engine="pyogrio")
    if source.empty:
        raise ValueError("input contains no features")
    if source.crs is None:
        raise ValueError("input must declare a coordinate reference system")
    source = source.to_crs("EPSG:4326")
    unsupported = sorted(
        {
            geometry.geom_type
            for geometry in source.geometry
            if geometry is not None
            and not geometry.is_empty
            and geometry.geom_type not in {"Polygon", "MultiPolygon"}
        }
    )
    if unsupported:
        raise TypeError(
            "input must contain only Polygon/MultiPolygon features; found "
            + ", ".join(unsupported)
        )
    if hasattr(source.geometry, "union_all"):
        geometry = source.geometry.union_all()
    else:
        geometry = source.geometry.unary_union
    frame = polygon_to_geodataframe(
        geometry,
        resolution,
        compact=compact,
        points_per_edge=points_per_edge,
        parallel=parallel,
        coverage_mode=coverage_mode,
    )
    write_geopackage(
        frame,
        output_path,
        layer=output_layer,
        overwrite=overwrite,
    )
    return frame


__all__ = [
    "cells_to_geodataframe",
    "geometry_to_cells",
    "polygon_file_to_geopackage",
    "polygon_to_geodataframe",
    "write_geopackage",
]
