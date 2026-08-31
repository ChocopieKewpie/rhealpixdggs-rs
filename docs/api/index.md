# API reference

The Python package exposes a compact H3-style functional API backed directly
by Rust. Functions are grouped here by the task they perform.

## Conventions

- Cell strings are canonical face-plus-digit identifiers such as `R88756047`.
- Scalar Python coordinates are `(latitude, longitude)` in degrees.
- Resolutions are integers from 0 through `MAX_RESOLUTION` (15).
- Invalid coordinates, resolutions, identifiers, or modes raise `ValueError`.
- Bulk NumPy IDs use `numpy.uint64`; scalar functions use cell strings unless
  their name explicitly converts to or from an integer.

```python
import rhealpixdggs as rh

print(rh.__version__)
print(rh.MAX_RESOLUTION)
```

## Surface by task

| Task | Main functions |
| --- | --- |
| [Indexing](indexing.md) | `latlng_to_cell`, `cell_to_latlng`, `cell_to_centroid` |
| [Cell geometry](geometry.md) | `cell_to_boundary_densified`, `cell_area`, `get_cell_shape` |
| [Hierarchy and IDs](hierarchy.md) | `cell_to_parent`, `cell_to_children`, `str_to_int` |
| [Topology](topology.md) | `cell_to_neighbors`, `grid_disk`, `cell_intersects` |
| [Coverage](coverage.md) | `bbox_to_cells`, `line_to_cells`, `polygon_to_cells_intersects` |
| [Compaction](compaction.md) | `compact_cells`, `uncompact_cells` |
| [NumPy](numpy.md) | `rhealpixdggs.numpy.latlngs_to_cells` |
| [GIS adapters](geo.md) | `geometry_to_cells`, `polygon_file_to_geopackage` |
| [Upstream facade](compat.md) | `RHEALPixDGGS`, `Cell`, `WGS84_003` |
| [Rust crate](rust.md) | `RhealpixDggs`, `CellId`, `Ellipsoid` |

## Public constants

### `MAX_RESOLUTION`

The finest resolution accepted by the stable 64-bit identifier encoding.
Currently `15`.

### `__version__`

The installed Python package version, currently `0.10.1` in this source tree.

