# `rhealpixdggs-py` compatibility facade

The object facade supports migration from the upstream Python project while
delegating projection and grid work to Rust.

```python
from rhealpixdggs import Cell, RHEALPixDGGS, WGS84_003
```

!!! warning "Coordinate order"
    This facade deliberately preserves upstream `(longitude, latitude)` order.
    It differs from the H3-style top-level Python functions.

## `WGS84_003`

Ready-made `RHEALPixDGGS` instance using WGS84, aperture 9 (`N_side=3`), and
polar-square positions `north_square=0`, `south_square=0`.

## `RHEALPixDGGS`

```python
RHEALPixDGGS(
    ellipsoid=None,
    north_square: int = 0,
    south_square: int = 0,
    N_side: int = 3,
)
```

Construct a configured aperture-9 grid. `WGS84_003` is the ready-made WGS84
configuration with both polar squares at position 0.

The main methods are grouped below.

| Group | Methods |
| --- | --- |
| Projection | `healpix`, `rhealpix`, `combine_triangles`, `triangle` |
| Point/cell conversion | `cell`, `cell_from_point`, `cell_from_region` |
| Coverage | `cells_from_region`, `cells_from_line`, `cells_from_meridian`, `cells_from_parallel`, `minimal_cover` |
| Iteration | `grid`, `interval`, `num_cells` |
| Metrics | `cell_width`, `cell_area`, `area_error_budget` |
| 3D helpers | `xyz`, `xyz_cube` |

### `RHEALPixDGGS.cell`

```python
dggs.cell(suid=None, level_order_index=None, post_order_index=None) -> Cell
```

Supply exactly one identifier source.

### `RHEALPixDGGS.cell_from_point`

```python
dggs.cell_from_point(resolution, point, plane=True) -> Cell | None
```

Use projected metres when `plane=True`, or `(longitude, latitude)` degrees
when `plane=False`.

### `RHEALPixDGGS.cells_from_region`

```python
dggs.cells_from_region(
    resolution,
    upper_left,
    lower_right,
    plane=True,
) -> list[list[Cell]]
```

Return the upstream-compatible row-structured region cover.

## `Cell`

```python
Cell(rdggs: RHEALPixDGGS, suid)
```

Usually construct cells through `dggs.cell("N45")`.

| Group | Methods/properties |
| --- | --- |
| Geometry | `nucleus`, `centroid`, `vertices`, `boundary`, `interior`, `ul_vertex`, `nw_vertex`, `xy_range` |
| Classification | `region`, `ellipsoidal_shape`, `north_square`, `south_square`, `N_side` |
| Hierarchy | `subcell`, `subcells`, `successor`, `predecessor`, `index`, `suid_rowcol` |
| Neighbours | `neighbor`, `neighbors`, `intersects_meridian`, `intersects_parallel` |
| Topology | `equals`, `within`, `contains`, `covers`, `covered_by`, `touches`, `disjoint`, `intersects`, `crosses`, `topologically_overlaps` |
| Metrics | `width`, `area` |
| Rotation | `rotate`, `rotate_entry` |

### Legacy overlap meaning

`Cell.overlaps(other)` retains the upstream hierarchical meaning: either cell
is an ancestor of the other. For OGC overlap semantics, use
`Cell.topologically_overlaps(other)` or top-level `cell_overlaps`.

The compatibility target and known differences are tracked in
[Upstream compatibility](../UPSTREAM_COMPATIBILITY.md).
