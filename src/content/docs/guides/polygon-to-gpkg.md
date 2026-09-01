---
title: Polygon to GeoPackage
description: Convert polygon coverage into an analysis-ready GeoPackage.
---

Convert a polygon dataset to GIS-ready rHEALPix cells with either centroid or
touch-all selection.

## Command line

```bash
rhealpix-to-gpkg input.gpkg cells.gpkg \
  --resolution 8 \
  --coverage-mode intersects \
  --points-per-edge 8 \
  --overwrite
```

The input can use any CRS recognized by GeoPandas/PROJ. It is reprojected to
EPSG:4326 and dissolved before coverage. The output geometry is stored in
EPSG:4326 and split safely at the antimeridian.

Use `--coverage-mode centroid` for upstream polyfill semantics. Use
`--coverage-mode intersects` when every touched cell must be included.

## Python

```python
from rhealpixdggs.geo import polygon_file_to_geopackage

frame = polygon_file_to_geopackage(
    "input.gpkg",
    "cells.gpkg",
    resolution=8,
    coverage_mode="intersects",
    points_per_edge=8,
    overwrite=True,
)

print(frame[["cell_id", "resolution", "area_m2"]].head())
```

Avoid `compact=True` when the immediate output is a map. Compacted coverage can
mix resolutions; uncompact to one resolution before rendering.
