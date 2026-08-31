# Rust crate API

The `rhealpixdggs` crate is the authoritative implementation used by the Python
extension. Public methods return `rhealpixdggs::Result<T>` for validation and
projection failures.

## Core types

### `RhealpixDggs`

Configured projection, indexing, geometry, topology, and coverage engine.

```rust
let dggs = RhealpixDggs::wgs84_003();
```

Method families:

| Family | Representative methods |
| --- | --- |
| Projection | `project_lonlat`, `unproject_lonlat`, `project_healpix_lonlat` |
| Indexing | `cell_from_lonlat`, `cell_from_projected`, `cell_to_lonlat` |
| Boundaries | `cell_vertices_lonlat`, `cell_boundary_lonlat`, `cell_boundary_projected` |
| Neighbours | `planar_neighbor`, `ellipsoidal_neighbor`, `ellipsoidal_neighbors` |
| Traversal | `grid_disk`, `grid_ring`, `are_neighbor_cells` |
| Coverage | `cells_from_region_lonlat`, `cells_from_polyline_lonlat`, `cells_from_polygon_lonlat`, `cells_from_polygon_lonlat_intersects` |
| Metrics | `cell_width`, `cell_area` |
| Bulk | `cells_from_lonlats_bulk`, `lonlats_from_cells_bulk`, `boundaries_lonlat_bulk`, `cells_from_bboxes_bulk` |

### `CellId`

Validated face plus aperture-9 digit path. It implements string parsing,
display, ordering, hierarchy, and stable `u64` conversion.

```rust
use std::str::FromStr;
use rhealpixdggs::CellId;

let cell = CellId::from_str("Q381")?;
assert_eq!(cell.resolution(), 3);
assert_eq!(cell.to_u64(), 3049);
assert_eq!(CellId::from_u64(3049)?, cell);
# Ok::<(), rhealpixdggs::Error>(())
```

Important methods include `parent`, `parent_at`, `child`, `children`,
`descendants`, `successor`, `predecessor`, `level_order_index`,
`post_order_index`, `to_u64`, and `from_u64`.

### `Ellipsoid`

```rust
let wgs84 = Ellipsoid::wgs84();
let sphere = Ellipsoid::sphere(6_371_008.8)?;
```

`WGS84_A` and `WGS84_INVERSE_FLATTENING` expose the defining WGS84 constants.

### Enums

| Type | Variants/purpose |
| --- | --- |
| `Face` | `N`, `O`, `P`, `Q`, `R`, `S` root faces |
| `Region` | north-polar, equatorial, south-polar classification |
| `CellShape` | quad, cap, dart, skew quad |
| `Direction` | unfolded-plane left/right/down/up |
| `EllipsoidalDirection` | shape-aware geographic direction names |
| `Error` | validated library failures |

## Free functions and constants

- `compact_cells` and `uncompact_cells` operate on `CellId` collections.
- `parallelism_available` reports whether the crate has Rayon support.
- `POINT_PARALLEL_THRESHOLD`, `BOUNDARY_PARALLEL_THRESHOLD`, and
  `REGION_PARALLEL_THRESHOLD` expose measured crossovers.
- `MAX_RESOLUTION` is 15.

Generate the complete item-by-item rustdoc locally:

```bash
cargo doc -p rhealpixdggs --no-deps --open
```

