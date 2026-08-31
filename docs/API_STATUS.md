# API and implementation status

The Rust crate is the source of truth. Python APIs are thin bindings or
adapters over the same projection, cell, topology, coverage, and metric code.

| Capability | Rust | Python | Verification |
|---|---:|---:|---|
| WGS84_003 point → cell | Yes | Yes | Upstream golden corpus |
| NumPy point/cell conversion | Ordered bulk | Yes | Scalar-equivalent |
| Batch boundaries and bounding boxes | Shared-edge bulk | Yes | Shared edges byte-identical |
| Projected/geographic nucleus and centroid | Yes | Yes | Golden and regression tests |
| Quad/cap/dart/skew-quad boundaries | Yes | Yes | All shapes and polar layouts |
| Planar and ellipsoidal neighbours | Yes | Yes | Seam and rotation tests |
| Grid disk/ring | Yes | Yes | Shortest-path topology tests |
| OGC-style cell predicates | Yes | Yes | Hierarchy and contact tests |
| Parent, children, descendants | Yes | Yes | Unit and corpus tests |
| Level/post-order traversal | Yes | Yes | Differential tests |
| String ↔ stable `u64` | Yes | Yes | Exhaustive low-resolution round trips |
| Equal-area cell metric | Yes | Yes | Golden tests |
| Rectangle and polyline coverage | Yes | Yes | Upstream coverage corpus |
| Polygon centroid coverage | Yes | Yes | Upstream parity plus seam fixes |
| Polygon intersection coverage | Yes | Yes | Interior, edge, corner, hole, seam, polar tests |
| Vector input → GeoPackage | Core coverage | `geo` extra | Optional integration tests |
| `RHEALPixDGGS` / `Cell` facade | Core-backed | Yes | Deterministic M1 surface |
| Custom aperture / `N_side` | Planned decision | No | — |

## Coverage rules

`cells_from_polygon_lonlat` / `polygon_to_cells` implement strict cell-centroid
containment for compatibility with upstream `polyfill` behavior.

`cells_from_polygon_lonlat_intersects` /
`polygon_to_cells_intersects` implement closed polygon-cell intersection.
Cell interior, edge-only, and corner-only contact all select the cell. A cell
wholly contained by a polygon hole is excluded. Both rules support optional
recursive compaction.

## Compatibility boundary

The completed M1 target is numerical and identifier parity with upstream
`WGS84_003`. The committed, language-neutral corpora are consumed by both Rust
and Python tests. Known upstream defects are corrected rather than reproduced;
see [UPSTREAM_COMPATIBILITY.md](UPSTREAM_COMPATIBILITY.md) and
[UPSTREAM_V0_7_AUDIT.md](UPSTREAM_V0_7_AUDIT.md).
