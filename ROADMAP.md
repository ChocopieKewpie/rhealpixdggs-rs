# Roadmap

The sequence is deliberately Python-first while keeping the core reusable by
other languages.

## M0 — Rust core and Python package (current)

- [x] Cargo workspace with a dependency-free core crate
- [x] PyO3/maturin Python package
- [x] Aperture-9 identifiers compatible with `WGS84_003`
- [x] Stable resolution-major `u64` encoding
- [x] Forward/inverse rHEALPix projection on spheres and WGS84
- [x] Point indexing, nuclei, planar corners, and equal cell areas
- [x] Parent/child, compact, and uncompact operations
- [x] Upstream golden examples and cross-platform CI definitions

## M1 — Upstream semantic parity

- [x] Import a versioned golden corpus from the released upstream and doctests
- [x] Cell shape classification: quad, cap, dart, skew quad
- [x] Geographic vertex ordering and optional dart trimming
- [x] Densified geographic boundaries with an explicit point-count contract
- [x] Planar edge neighbours, including polar rotations
- [x] Ellipsoidal edge-neighbour direction names
- [x] Cell ordering, predecessor/successor, and level/post-order indices
- [x] Region, line, and polygon coverage without geometry types in core
- [x] Initial Python `RHEALPixDGGS` and `Cell` compatibility facade
- [ ] Complete facade parity for ordering, traversal, and geometry methods

Exit criterion: documented supported calls match upstream IDs and numerical
outputs within an agreed error budget over random, boundary, antimeridian, and
polar fixtures.

## M2 — Performance API

- [ ] Criterion baseline against released `rhealpixdggs-py`
- [ ] NumPy array input/output without per-point Python calls
- [ ] Release the Python GIL around bulk operations
- [ ] Optional Rayon parallelism above a measured crossover size
- [ ] Batch cell-to-boundary and region-cover operations
- [ ] Benchmark memory, latency, and throughput on Linux, Windows, and macOS

Performance claims will be published only with reproducible fixtures and both
single-point and bulk measurements.

## M3 — Additional language bindings

- [ ] Stable C ABI over integer/string IDs and flat coordinate buffers
- [ ] WebAssembly/JavaScript package
- [ ] Bindings selected from real users: likely R, Java, or C# next
- [x] Shared conformance fixtures consumed by the core and implemented bindings

## M4 — Standards and ecosystem

- [ ] Track OGC Topic 21 / ISO 19170-1 conformance work
- [ ] GeoArrow and Apache Arrow batch interfaces
- [ ] GeoJSON/WKT adapters outside the dependency-free core
- [ ] PyPI, crates.io, conda-forge, and platform wheel release automation

## Open design decisions

1. Keep the fast path fixed to aperture 9, or generalise identifiers for
   arbitrary upstream `N_side` values.
2. Whether the cross-language public integer ID remains resolution-major or
   moves to a reserved bit layout before 1.0. String IDs remain canonical.
3. Whether high-level polygon coverage belongs here or in a companion crate
   integrating GEOS/geo/GeoArrow.
