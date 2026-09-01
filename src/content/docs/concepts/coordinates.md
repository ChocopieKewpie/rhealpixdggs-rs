---
title: Coordinates and boundaries
description: Work correctly with coordinate order, projections, and geographic boundaries.
---

The project exposes three coordinate conventions because it serves different
ecosystems. Mixing them is the most common source of indexing mistakes.

| Surface | Geographic order | Units |
| --- | --- | --- |
| H3-style Python functions | `(latitude, longitude)` | degrees |
| Python `RHEALPixDGGS` / `Cell` facade | `(longitude, latitude)` | degrees |
| Rust `RhealpixDggs` methods | `(longitude, latitude)` | degrees |
| Projected coordinates | `(x, y)` | metres |

## Nucleus and centroid

`cell_to_latlng(cell)` returns the geographic coordinate obtained by inverse
projecting the planar cell nucleus. `cell_to_centroid(cell)` returns the
ellipsoidal centroid used by centroid-based polygon coverage.

The distinction matters most for non-standard polar shapes. Do not assume that
the nucleus is the geometric centroid of a rendered geographic polygon.

## Boundary functions

`cell_to_boundary` returns the defining vertices. For mapping, curved edges
usually need additional points:

```python
boundary = rh.cell_to_boundary_densified(
    "R88756047",
    points_per_edge=16,
)
```

The output contains exactly `4 * points_per_edge - 4` coordinates. The ring is
not closed by repeating its first point.

Set `interior=True` when a boundary must sit just inside the mathematical cell
edge, for example to avoid ambiguous rasterization of shared edges. It does not
change cell topology.

## Antimeridian-safe GIS polygons

Raw geographic boundaries can cross ±180°. The optional GeoPandas adapter
splits such cells into a `MultiPolygon`, keeping every part within the ordinary
`[-180°, 180°]` longitude range:

```python
from rhealpixdggs.geo import cells_to_geodataframe

frame = cells_to_geodataframe(["O8"], points_per_edge=16)
assert frame.crs.to_epsg() == 4326
```

![Geographic cells and polar convergence](/rhealpixdggs-rs/images/geographic-faces.svg)
