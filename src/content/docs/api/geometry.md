---
title: Cell geometry
description: Inspect cell centroids, boundaries, areas, and geographic shapes.
---

Geometry functions describe cells on WGS84_003. Python outputs use
`(latitude, longitude)` order.

## `cell_to_boundary`

```python
cell_to_boundary(
    cell: str,
    trim_dart: bool = False,
) -> list[tuple[float, float]]
```

Return the defining geographic vertices. Set `trim_dart=True` to use the
upstream-compatible trimmed representation for dart cells. The returned ring
is open: the first point is not repeated at the end.

## `cell_to_boundary_densified`

```python
cell_to_boundary_densified(
    cell: str,
    points_per_edge: int = 2,
    interior: bool = False,
) -> list[tuple[float, float]]
```

Sample every projected cell edge before inverse projection. The result always
contains `4 * points_per_edge - 4` points. `points_per_edge` must be at least 2.

`interior=True` applies a small inward inset suitable for edge-sensitive
rasterization; it does not define a different cell.

## `cells_to_boundaries`

```python
cells_to_boundaries(
    cells: Sequence[str],
    points_per_edge: int = 2,
    interior: bool = False,
    parallel: bool | None = None,
) -> list[list[tuple[float, float]]]
```

Return boundaries in input order. Shared edge work is deduplicated. With
`parallel=None`, the implementation selects parallel execution at the measured
crossover; `True` forces Rayon and `False` uses the calling thread.

For very large collections already stored as `uint64`, use
[`rhealpixdggs.numpy.cells_to_boundaries`](/rhealpixdggs-rs/api/numpy/#cells_to_boundaries).

## `get_cell_region`

```python
get_cell_region(cell: str) -> str
```

Return one of `north_polar`, `equatorial`, or `south_polar`.

## `get_cell_shape`

```python
get_cell_shape(cell: str) -> str
```

Return one of `quad`, `cap`, `dart`, or `skew_quad`.

## `cell_area`

```python
cell_area(cell: str, unit: str = "m^2") -> float
```

Return equal-area cell area. Accepted units are `m^2`, `m2`, `km^2`, and
`km2`. Area depends on resolution, not face or child digits.

```python
assert rh.cell_area("Q4", "km2") == rh.cell_area("S7", "km2")
```
