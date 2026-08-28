# Architecture

## Boundary between core and bindings

`rhealpixdggs` owns all projection, identifier, hierarchy, and grid logic. It
uses Rust primitives and fixed-size coordinate tuples only. It does not know
about Python, NumPy, Shapely, GEOS, Arrow, or serialization formats.

Bindings translate their language's values into:

- `CellId` or its stable `u64` form;
- longitude/latitude `f64` values;
- flat slices for future batch APIs.

This keeps correctness fixtures reusable and prevents Python object allocation
from shaping the core API.

## Identifiers

Canonical strings use one face letter (`N`, `O`, `P`, `Q`, `R`, `S`) followed
by zero or more aperture-9 child digits (`0` through `8`). Digits are row-major
within each 3×3 parent.

The stable integer form is the level-order, resolution-major ordinal:

```text
offset(resolution) + face × 9^resolution + base9(child_digits)
```

where `offset(r) = 6 × (9^r − 1) / 8`. All cells through resolution 15 fit
comfortably in `u64`. This encoding is explicitly alpha until 1.0.

The core also exposes a post-order ordinal over the complete hierarchy through
resolution 15. Cell comparison uses that order: descendants sort before their
parent, then child subtrees `0..=8`, then faces `N..S`. Keeping the two orders
explicit avoids overloading the stable interchange ID with traversal behavior.

## Coordinates

The core uses `(longitude, latitude)` for geographic coordinates and metres for
projected coordinates. Angles at the public Rust boundary are degrees. The
H3-style Python functions swap to `(latitude, longitude)` ordering. The
upstream-compatibility `RHEALPixDGGS`/`Cell` facade deliberately retains
`(longitude, latitude)` so existing callers do not need coordinate shims.

## Topology and shape semantics

Region and ellipsoidal-shape classification live on `CellId` because they are
pure functions of the face and aperture-9 child digits. Neighbour traversal
lives on `RhealpixDggs` because polar-square placement changes resolution-zero
face adjacency. Geographic vertex ordering likewise belongs to the DGGS: polar
skew quadrilaterals and darts require projection-triangle context. Geographic
neighbour names are derived from neighbour nuclei; circular longitude offsets
relative to the source-cell nucleus avoid antimeridian special cases and
mutable prime-meridian state.

## Geometry coverage

Coverage accepts ordinary coordinate slices rather than GEOS, Shapely, or
`geo` objects. Rectangle scans preserve upstream row ordering. Polyline cover
returns every touched cell in path order, and polygon cover uses strict cell-
centroid containment with optional holes and recursive compaction. Geographic
longitudes are unwrapped internally, so antimeridian-crossing lines and rings
take the short path. Polar caps are tested as latitude-bounded regions instead
of being forced into invalid longitude polygons.

This boundary keeps the core reusable from any language. Bindings can add
GeoJSON, Shapely, or ecosystem-specific adapters without making those object
models part of the grid algorithm.

## Correctness strategy

1. Small unit tests cover algebraic invariants and projection round trips.
2. A checked-in, versioned JSON corpus records exact upstream cell IDs and
   tolerant floating outputs with source-file hashes and a JSON Schema.
3. Both the Rust integration suite and Python suite consume that same corpus;
   future language bindings must do the same.
4. The deterministic generator runs against an exact upstream release. A new
   upstream release creates a new immutable corpus directory rather than
   silently changing an existing compatibility contract.
5. Differential tests will extend the corpus with random and additional
   edge-case inputs as new operations land.

## Performance strategy

The initial core is dependency-free and allocation-light for single-point
indexing. Bulk interfaces will accept contiguous coordinate and output buffers.
Parallelism will be optional and introduced only where benchmarks show a net
benefit; native code alone does not make Python-to-Rust call overhead free.
