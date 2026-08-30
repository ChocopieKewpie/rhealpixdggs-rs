# Changelog

## 0.8.1 (unreleased)

- Made polygon ring-area validation robust for tiny and sliver fragments by
  translating coordinates before area accumulation and using a scale-relative
  area tolerance instead of the shared geometry epsilon.
- Added global regression sweeps proving that tiny polygon validation remains
  stable across coordinate translations, scale changes, ring orientation, the
  antimeridian, and both polar regions.

## 0.8.0

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
- Added an optional GeoPandas/Shapely adapter for Polygon/MultiPolygon coverage,
  bulk cell-boundary construction, antimeridian-safe geometry, and GeoPackage
  output.
- Added a detailed SimpleMaps New Zealand MultiPolygon (CC BY 4.0) as the
  Rust-scale resolution-8 workload, alongside a deterministic lightweight
  resolution-6 fixture for the isolated benchmark against `rHEALPixDGGS`
  0.6.0.
- Recorded the matched Windows polygon result: identical 1,859-cell output,
  4,974× faster coverage, and 724× faster end-to-end conversion in Rust 0.8.0
  than upstream Python 0.6.0 (one measured sample after warm-up).
- Fixed CI and benchmark workflows to build and install wheels instead of
  calling `maturin develop` without an activated virtual environment, and
  added a dedicated GeoPackage adapter test job.
- Kept the facade corpus tests compatible with the package's Python 3.9
  minimum by avoiding the Python 3.10-only `zip(strict=...)` argument.
- Forced versioned corpus fixtures to retain LF line endings on Windows so
  their byte-level SHA-256 provenance checks are reproducible in CI.
- Stabilized the GIL-release concurrency test on fast Windows runners by
  removing its startup sleep and using a longer single Rust bulk operation.
- Recorded the Windows x86_64 M2 result and explicitly deferred macOS arm64 as
  TBD until suitable hardware or a hosted run is available.

## 0.1.0-alpha.1

- Created a dependency-free Rust aperture-9 rHEALPix core.
- Added WGS84 projection, point indexing, nuclei, corners, and area metrics.
- Added canonical string and stable alpha `u64` cell identifiers.
- Added hierarchy compaction/expansion operations.
- Added an H3-style Python API using PyO3 and maturin.
- Added upstream golden cases, CI, benchmarks, type stubs, and a parity roadmap.
