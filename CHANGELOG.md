# Changelog

## Unreleased

- Added region and ellipsoidal shape classification in the Rust core.
- Added upstream-compatible geographic vertex ordering and optional dart
  trimming.
- Added planar edge neighbours with polar-face rotations and configurable
  north/south square positions.
- Added H3-style Python neighbour and shape helpers.
- Added an initial upstream-style Python `RHEALPixDGGS`, `Cell`, and
  `WGS84_003` compatibility facade.

## 0.1.0-alpha.1

- Created a dependency-free Rust aperture-9 rHEALPix core.
- Added WGS84 projection, point indexing, nuclei, corners, and area metrics.
- Added canonical string and stable alpha `u64` cell identifiers.
- Added hierarchy compaction/expansion operations.
- Added an H3-style Python API using PyO3 and maturin.
- Added upstream golden cases, CI, benchmarks, type stubs, and a parity roadmap.
