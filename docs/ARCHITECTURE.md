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

The integer form is a resolution-major ordinal:

```text
offset(resolution) + face × 9^resolution + base9(child_digits)
```

where `offset(r) = 6 × (9^r − 1) / 8`. All cells through resolution 15 fit
comfortably in `u64`. This encoding is explicitly alpha until 1.0.

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
skew quadrilaterals and darts require projection-triangle context.

## Correctness strategy

1. Small unit tests cover algebraic invariants and projection round trips.
2. Golden tests record exact upstream cell IDs and tolerant floating outputs.
3. Differential tests will run random and edge-case inputs through both
   implementations.
4. Every future language binding consumes the same fixture corpus.

## Performance strategy

The initial core is dependency-free and allocation-light for single-point
indexing. Bulk interfaces will accept contiguous coordinate and output buffers.
Parallelism will be optional and introduced only where benchmarks show a net
benefit; native code alone does not make Python-to-Rust call overhead free.
