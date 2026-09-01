---
title: GeoPandas and GeoPackage API
description: Convert geometries to cells and export GIS-ready GeoPackages.
---

```python
from rhealpixdggs import geo as rhgeo
```

Install the optional dependencies with `pip install "rhealpixdggs-rs[geo]"`.
Input geometries must be valid Shapely `Polygon` or `MultiPolygon` objects.

## `geometry_to_cells`

```python
rhgeo.geometry_to_cells(
    geometry,
    resolution: int,
    *,
    compact: bool = False,
    coverage_mode: Literal["centroid", "intersects"] = "centroid",
) -> list[str]
```

Cover an EPSG:4326 polygonal geometry. MultiPolygon parts are unioned at the
cell-ID level, so duplicate cells are removed. `intersects` is touch-all mode.

## `cells_to_geodataframe`

```python
rhgeo.cells_to_geodataframe(
    cells,
    *,
    points_per_edge: int = 4,
    parallel: bool | None = None,
) -> geopandas.GeoDataFrame
```

Create an EPSG:4326 frame with `cell_id`, `resolution`, `area_m2`, and
`geometry` columns. Geometry is always stored as `MultiPolygon`; cells crossing
the antimeridian are split at ±180° for conventional GIS display.

## `polygon_to_geodataframe`

```python
rhgeo.polygon_to_geodataframe(
    geometry,
    resolution: int,
    *,
    compact: bool = False,
    points_per_edge: int = 4,
    parallel: bool | None = None,
    coverage_mode: Literal["centroid", "intersects"] = "centroid",
) -> geopandas.GeoDataFrame
```

Combine `geometry_to_cells` and `cells_to_geodataframe` in one call.

## `write_geopackage`

```python
rhgeo.write_geopackage(
    frame,
    output,
    *,
    layer: str = "rhealpix_cells",
    overwrite: bool = False,
) -> pathlib.Path
```

Write through GeoPandas/Pyogrio. Existing outputs are protected unless
`overwrite=True`.

## `polygon_file_to_geopackage`

```python
rhgeo.polygon_file_to_geopackage(
    input_path,
    output_path,
    resolution: int,
    *,
    input_layer: str | None = None,
    output_layer: str = "rhealpix_cells",
    compact: bool = False,
    points_per_edge: int = 4,
    parallel: bool | None = None,
    overwrite: bool = False,
    coverage_mode: Literal["centroid", "intersects"] = "centroid",
) -> geopandas.GeoDataFrame
```

Read polygon features, require a declared CRS, reproject to EPSG:4326,
dissolve all features, cover the result, write a GeoPackage, and return the
written frame.
