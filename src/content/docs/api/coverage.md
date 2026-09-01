---
title: Region coverage
description: Cover boxes, lines, and polygons with rHEALPix cells.
---

All coordinate sequences on this page use `(latitude, longitude)` degrees.

## `bbox_to_cells`

```python
bbox_to_cells(
    north: float,
    south: float,
    east: float,
    west: float,
    resolution: int,
) -> list[str]
```

Return cells covering a latitude/longitude bounding box. Use `west > east` for
an antimeridian-crossing box.

## `line_to_cells`

```python
line_to_cells(
    coordinates: Sequence[tuple[float, float]],
    resolution: int,
) -> list[str]
```

Return every cell touched by a polyline in path order. Consecutive duplicate
cells are removed. Antimeridian segments follow the short route.

## `polygon_to_cells`

```python
polygon_to_cells(
    exterior: Sequence[tuple[float, float]],
    resolution: int,
    holes: Sequence[Sequence[tuple[float, float]]] | None = None,
    compact: bool = False,
) -> list[str]
```

Select cells whose ellipsoidal centroids fall inside the polygon. Boundary
rings can be open or closed and use either winding direction. Hole interiors
are excluded.

## `polygon_to_cells_intersects`

```python
polygon_to_cells_intersects(
    exterior: Sequence[tuple[float, float]],
    resolution: int,
    holes: Sequence[Sequence[tuple[float, float]]] | None = None,
    compact: bool = False,
) -> list[str]
```

Select every cell whose closed geographic geometry intersects the polygon,
including edge and corner contact. This touch-all mode is useful for narrow
features and conservative candidate generation.

```python
centroid = rh.polygon_to_cells(exterior, 8)
touch_all = rh.polygon_to_cells_intersects(exterior, 8)
assert set(centroid) <= set(touch_all)
```

See [Coverage semantics](/rhealpixdggs-rs/concepts/coverage/) for mode selection,
compaction, polar handling, and rendering advice.
