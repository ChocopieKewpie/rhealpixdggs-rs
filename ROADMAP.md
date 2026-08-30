# Roadmap

The sequence is deliberately Python-first while keeping the core reusable by
other languages.

## M0 — Rust core and Python package

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
- [x] Complete deterministic facade parity for ordering, traversal, and geometry methods

Exit criterion met: documented supported calls match upstream IDs and
numerical outputs within explicit error budgets over generated point,
boundary, antimeridian, polar, traversal, coverage, and facade fixtures. See
[`docs/UPSTREAM_COMPATIBILITY.md`](docs/UPSTREAM_COMPATIBILITY.md).

## M2 — Performance API (macOS measurement deferred)

- [x] Criterion core baseline and reproducible comparison with released `rhealpixdggs-py` 0.6.0
- [x] NumPy array input/output without per-point Python calls
- [x] Release the Python GIL around bulk operations
- [x] Optional Rayon parallelism above measured operation-specific crossover sizes
- [x] Batch cell-to-boundary and region-cover operations
- [x] Reproducible New Zealand polygon → GeoPackage comparison harness
- [x] Record the 0.8.0 versus 0.6.0 New Zealand result on Windows
- [ ] Benchmark memory, latency, and throughput on all target operating systems
  - [x] Linux x86_64
  - [x] Windows x86_64
  - [ ] macOS arm64 — **TBD (hardware unavailable)**

The JSON-emitting Python harness now has published Linux and Windows records.
The macOS arm64 run remains visible as a deferred portability measurement and
does not block work on the next milestones. The manual workflow remains ready
for a future macOS runner.

Performance claims will be published only with reproducible fixtures and both
single-point and bulk measurements.

## M2.1 — Core numerical hardening

- [x] Use translation-stable polygon area accumulation
- [x] Replace the shared geometry epsilon with a scale-relative area tolerance
- [x] Exercise tiny rings across global translations and representable scales
- [x] Cover reversed orientation, thin slivers, the antimeridian, and both poles
- [ ] Add property-based generation and a persistent fuzzing corpus
- [ ] Publish the 0.8.1 Python wheels and Rust crate

## M3 — Core topology and identifier contract

- [x] Add `grid_disk` and `grid_ring` traversal across face boundaries
- [x] Add a same-resolution edge-neighbour predicate
- [ ] Add distance and path operations
- [ ] Freeze and document the cross-language `u64` bit/range contract
- [ ] Publish language-neutral identifier and topology test vectors
- [ ] Decide whether 1.0 remains fixed to aperture 9

## M4 — Additional language bindings

- [ ] Stable C ABI over integer/string IDs and flat coordinate buffers
- [ ] WebAssembly/JavaScript package
- [ ] Bindings selected from real users: likely R, Java, or C# next
- [x] Shared conformance fixtures consumed by the core and implemented bindings

## M5 — Standards and ecosystem

- [ ] Track OGC Topic 21 / ISO 19170-1 conformance work
- [ ] GeoArrow and Apache Arrow batch interfaces
- [x] GeoPandas vector input and GeoPackage output outside the dependency-free core
- [ ] Direct GeoJSON/WKT and streaming adapters
- [ ] PyPI, crates.io, conda-forge, and platform wheel release automation

## Open design decisions

1. Keep the fast path fixed to aperture 9, or generalise identifiers for
   arbitrary upstream `N_side` values.
2. Whether the cross-language public integer ID remains resolution-major or
   moves to a reserved bit layout before 1.0. String IDs remain canonical.
3. Whether future high-level geometry operations remain Python adapters or
   move into a companion Rust crate integrating `geo`/GEOS/GeoArrow.
