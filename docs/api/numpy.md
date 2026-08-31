# NumPy API

```python
from rhealpixdggs import numpy as rhnp
```

NumPy operations accept C-contiguous numeric arrays, cross the extension
boundary once, release the GIL, and return read-only views over Rust-produced
buffers. Geographic arrays use `(latitude, longitude)` order.

## `latlngs_to_cells`

```python
rhnp.latlngs_to_cells(
    coordinates,
    resolution: int,
    *,
    parallel: bool | None = None,
) -> numpy.ndarray[numpy.uint64]
```

`coordinates` must have shape `(n, 2)`. The result has shape `(n,)` and
contains stable integer cell IDs.

## `cells_to_latlngs`

```python
rhnp.cells_to_latlngs(
    cells,
    *,
    parallel: bool | None = None,
) -> numpy.ndarray[numpy.float64]
```

Convert a one-dimensional integer ID array into cell nuclei with shape
`(n, 2)`.

## `cells_to_boundaries`

```python
rhnp.cells_to_boundaries(
    cells,
    *,
    points_per_edge: int = 2,
    interior: bool = False,
    parallel: bool | None = None,
) -> numpy.ndarray[numpy.float64]
```

Return an array with shape `(n, 4 * points_per_edge - 4, 2)`. The final axis is
`(latitude, longitude)`.

## `bboxes_to_cells`

```python
rhnp.bboxes_to_cells(
    bboxes,
    resolution: int,
    *,
    parallel: bool | None = None,
) -> tuple[numpy.ndarray[numpy.uint64], numpy.ndarray[numpy.uint64]]
```

Each input row is `(north, south, east, west)`. Because each box can produce a
different number of cells, output uses compressed sparse row-style offsets:

```python
cells, offsets = rhnp.bboxes_to_cells(boxes, 8)
cells_for_box_i = cells[offsets[i] : offsets[i + 1]]
```

A row with `west > east` crosses the antimeridian.

## Parallelism

Every function accepts `parallel`:

| Value | Behaviour |
| --- | --- |
| `None` | Select Rayon only at the measured operation-specific crossover. |
| `True` | Force Rayon when the installed build supports it. |
| `False` | Run on the calling thread. |

### `PARALLELISM_AVAILABLE`

Boolean indicating whether this extension was built with parallel support.

### `POINT_PARALLEL_THRESHOLD`

Measured automatic crossover for point indexing and inverse indexing.

### `BOUNDARY_PARALLEL_THRESHOLD`

Measured automatic crossover for boundary generation.

### `REGION_PARALLEL_THRESHOLD`

Measured automatic crossover for batched bounding-box coverage.

Treat thresholds as implementation guidance rather than API promises; they can
change when benchmarks justify a new crossover.

