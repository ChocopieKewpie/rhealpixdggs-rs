---
title: Topology and traversal
description: Find neighbours, disks, rings, and paths across rHEALPix seams.
---

Topology is defined by the global four-edge neighbour graph, not by distance
between polygons in the unfolded map.

## `cell_to_neighbor`

```python
cell_to_neighbor(
    cell: str,
    direction: str,
    plane: bool = True,
) -> str | None
```

Return one same-resolution edge neighbour.

With `plane=True`, directions are `left`, `right`, `down`, and `up`. With
`plane=False`, names follow geographic orientation and depend on cell shape:
quadrilaterals use `north`, `south`, `east`, and `west`; polar shapes may use
diagonal or indexed poleward names. A valid ellipsoidal direction can return
`None` where a shape has no neighbour under that name.

## `cell_to_neighbors`

```python
cell_to_neighbors(cell: str, plane: bool = True) -> dict[str, str]
```

Return all four edge neighbours as a direction-to-cell mapping.

## `are_neighbor_cells`

```python
are_neighbor_cells(origin: str, destination: str) -> bool
```

Return whether two same-resolution cells share an edge. Cells that touch only
at a corner are not neighbours.

## `grid_disk`

```python
grid_disk(origin: str, k: int) -> list[str]
```

Return the origin and all cells within `k` shortest edge steps. The origin is
first; subsequent distance layers use deterministic canonical ordering.

## `grid_ring`

```python
grid_ring(origin: str, k: int) -> list[str]
```

Return cells whose shortest edge distance from the origin is exactly `k`.
`grid_ring(origin, 0)` contains only the origin.

![Actual traversal across rHEALPix seams](/rhealpixdggs-rs/images/edge-traversal-gis.svg)

## Cell predicates

All predicate functions accept two canonical cell strings:

```python
predicate(left: str, right: str) -> bool
```

| Function | Meaning |
| --- | --- |
| `cell_equals` | Both identifiers describe the same cell. |
| `cell_within` | The first cell is hierarchically within the second. |
| `cell_contains` | The first cell hierarchically contains the second. |
| `cell_covers` | The first cell covers the second under closed-cell semantics. |
| `cell_covered_by` | The first cell is covered by the second. |
| `cell_touches` | Boundaries share a point but interiors do not overlap. |
| `cell_disjoint` | The closed cells share no point. |
| `cell_intersects` | The closed cells share at least one point. |
| `cell_crosses` | OGC `crosses` predicate; false for the hierarchy cases represented here. |
| `cell_overlaps` | OGC topological overlap predicate, distinct from legacy facade overlap. |

### `cell_equals`

### `cell_within`

### `cell_contains`

### `cell_covers`

### `cell_covered_by`

### `cell_touches`

### `cell_disjoint`

### `cell_intersects`

### `cell_crosses`

### `cell_overlaps`

The individual headings above provide stable link targets for each public
function; their signatures and semantics are summarized in the table.
