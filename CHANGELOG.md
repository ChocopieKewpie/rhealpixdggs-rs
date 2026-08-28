# Changelog

## Unreleased

- Added region and ellipsoidal shape classification in the Rust core.
- Added upstream-compatible geographic vertex ordering and optional dart
  trimming.
- Added planar edge neighbours with polar-face rotations and configurable
  north/south square positions.
- Added ellipsoidal edge-neighbour direction semantics for quads, caps, darts,
  and skew quads.
- Added H3-style Python neighbour and shape helpers.
- Added an initial upstream-style Python `RHEALPixDGGS`, `Cell`, and
  `WGS84_003` compatibility facade.
- Completed the deterministic M1 facade with projection helpers, region-parent
  selection, latitude and meridian/parallel traversal, cell geometry
  predicates, rotations, interior grids, and overlap operations.
- Added a shared 63-case upstream facade corpus consumed by the Rust and Python
  suites, with explicit projected and geographic error budgets.
- Updated package authorship to James Ardo and repository metadata to
  `ChocopieKewpie/rhealpixdggs-rs`.
- Added ordered Rust bulk point, nucleus, densified-boundary, and bounding-box
  coverage operations.
- Added NumPy integer-ID APIs using one contiguous extension call per batch,
  with GIL release and read-only result views.
- Added optional Rayon execution with Criterion-measured automatic crossover
  thresholds for point, boundary, and region workloads.
- Added reproducible Linux memory/latency/throughput comparisons against
  `rhealpixdggs-py` 0.6.0 and a manual three-platform benchmark workflow.
- Documented and regression-tested the numerically stable authalic-latitude
  series evaluated by Gilić and Gašparović (2025).

## 0.1.0-alpha.1

- Created a dependency-free Rust aperture-9 rHEALPix core.
- Added WGS84 projection, point indexing, nuclei, corners, and area metrics.
- Added canonical string and stable alpha `u64` cell identifiers.
- Added hierarchy compaction/expansion operations.
- Added an H3-style Python API using PyO3 and maturin.
- Added upstream golden cases, CI, benchmarks, type stubs, and a parity roadmap.
