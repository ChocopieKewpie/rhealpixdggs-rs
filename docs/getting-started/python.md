# Python quickstart

This walkthrough covers the core operations without requiring GeoPandas.

## Index a coordinate

```python
import rhealpixdggs as rh

latitude = -40.356
longitude = 175.611
resolution = 8

cell = rh.latlng_to_cell(latitude, longitude, resolution)
print(cell)  # R88756047
```

The first letter is the root face. Each remaining digit selects one child in a
row-major 3×3 subdivision. The number of digits is the resolution.

## Inspect the cell

```python
nucleus = rh.cell_to_latlng(cell)
centroid = rh.cell_to_centroid(cell)
boundary = rh.cell_to_boundary_densified(cell, points_per_edge=8)
area_km2 = rh.cell_area(cell, "km2")

print(nucleus)
print(centroid)
print(area_km2)  # all resolution-8 cells have the same area
```

`cell_to_latlng` returns the inverse-projected planar nucleus.
`cell_to_centroid` returns the geographic centroid used by centroid coverage.
For ordinary cells the two are close, but they are not defined identically.

## Walk the hierarchy

```python
parent = rh.cell_to_parent(cell)
siblings = rh.cell_to_children(parent)

assert cell in siblings
assert rh.get_resolution(parent) == 7
assert len(siblings) == 9
```

Pass an explicit resolution to retrieve a more distant ancestor or every
descendant at a deeper level:

```python
face = rh.cell_to_parent(cell, resolution=0)
descendants = rh.cell_to_children(face, resolution=2)
assert len(descendants) == 81
```

## Traverse neighbours

```python
neighbors = rh.cell_to_neighbors(cell, plane=False)
nearby = rh.grid_disk(cell, k=2)
ring = rh.grid_ring(cell, k=2)

assert cell in nearby
assert cell not in ring
```

Use `plane=False` when direction names should describe geographic rather than
unfolded-projection directions. `grid_disk` and `grid_ring` always use the
continuous geographic edge graph.

## Stable integer IDs

```python
cell_u64 = rh.str_to_int(cell)
assert rh.int_to_str(cell_u64) == cell
```

The integer mapping is deterministic and resolution-major. It is suitable for
`uint64` arrays and database columns; it is not an H3-style packed bitfield.

## Index a NumPy array

```python
import numpy as np
from rhealpixdggs import numpy as rhnp

points = np.array([
    [-40.356, 175.611],
    [-41.2865, 174.7762],
    [40.7128, -74.0060],
])

cell_ids = rhnp.latlngs_to_cells(points, resolution=8)
centers = rhnp.cells_to_latlngs(cell_ids)
boundaries = rhnp.cells_to_boundaries(cell_ids, points_per_edge=8)

print(cell_ids.dtype)      # uint64
print(boundaries.shape)    # (3, 28, 2)
```

Bulk calls cross the Python/Rust boundary once, release the GIL, and use Rayon
when the workload exceeds measured crossover thresholds.

## Next steps

- [Cover lines and polygons](../concepts/coverage.md)
- [Aggregate Waka Kotahi CAS crash points](../recipes/cas-crash-density.md)
- [Browse the Python API by task](../api/index.md)
