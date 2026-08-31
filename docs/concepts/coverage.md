# Coverage semantics

Coverage converts a region or path into cells at one resolution. The library
keeps the selection rule explicit because different analyses need different
answers.

## Bounding boxes

```python
cells = rh.bbox_to_cells(
    north=-40.0,
    south=-41.0,
    east=176.0,
    west=175.0,
    resolution=8,
)
```

If `west > east`, the box crosses the antimeridian and is split automatically.

## Lines

`line_to_cells` returns every cell touched by a latitude/longitude polyline in
path order. Antimeridian segments take the short geographic route.

```python
cells = rh.line_to_cells(
    [(-41.29, 174.78), (-40.36, 175.61)],
    resolution=8,
)
```

## Polygon modes

=== "Centroid"

    ```python
    cells = rh.polygon_to_cells(exterior, 8, holes=holes)
    ```

    A cell is selected when its geographic centroid is inside the polygon.
    This preserves `rhealpixdggs-py` polyfill semantics and gives disjoint
    assignments when adjacent polygons share an edge.

=== "Intersects / touch-all"

    ```python
    cells = rh.polygon_to_cells_intersects(exterior, 8, holes=holes)
    ```

    A cell is selected when its closed geometry has any point in common with
    the polygon, including edge-only and corner-only contact. It is therefore
    normally a superset of centroid coverage.

Coordinates use Python `(latitude, longitude)` order. Rings may be open or
closed and can use either winding direction. Holes, antimeridian crossings,
polar regions, and very thin valid fragments are supported.

## Compacted output

Both polygon functions accept `compact=True`. A complete group of nine sibling
cells is replaced recursively by its parent. This is excellent for storage,
but it produces mixed resolutions:

```python
compact = rh.polygon_to_cells(exterior, 8, compact=True)
display_cells = rh.uncompact_cells(compact, 8)
```

Uncompact before rendering a uniform-resolution grid. Geographic boundaries
of parent and child cells do not nest as exact planar polygons after inverse
projection, so drawing a mixed cover can show visual overlaps or gaps.

## Choosing a mode

| Need | Recommended mode |
| --- | --- |
| Assign each event to exactly one polygon | Centroid |
| Preserve historical `rhealpixdggs-py` output | Centroid |
| Find every cell potentially affected by a feature | Intersects |
| Avoid missing narrow/sliver intersections | Intersects |
| Produce a conservative search candidate set | Intersects |

