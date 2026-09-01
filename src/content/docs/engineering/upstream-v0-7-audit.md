---
title: Upstream rhealpixdggs-py v0.7.0 issue audit
description: Audit upstream milestone issues against the current Rust implementation.
---

Audited on 2026-08-31 against the 20 open issues in the upstream
[`v0.7.0` milestone](https://github.com/manaakiwhenua/rhealpixdggs-py/milestone/2)
and Rust base commit `089cbcba78ada1f047877e4a6448eeb422f3a833`.

The audit ports behavior and correctness requirements, not Python-specific
implementation details. The Rust core remains dependency-free and fixed to the
documented aperture-9 identifier contract.

| Issue | Disposition in `rhealpixdggs-rs` |
|---|---|
| [#87 Batch boundary shared-edge deduplication](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/87) | **Implemented in 0.10.0.** Non-inset bulk boundaries project every unique edge once, reuse it in reverse for its neighbour, preserve input order, and make shared copies byte-identical. Inset boundaries retain the scalar/parallel path because they do not share edges. |
| [#84 Manaaki Whenua standards Action](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/84) | **Repository-specific; not applicable.** This repository is not in that organisation. It already uses `README.md`, has CI, and declares its licence and metadata. |
| [#80 Sphinx theme, summaries, and figures](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/80) | **Implementation-specific; not ported.** The Rust project uses Markdown and generated Rust API documentation rather than the upstream Sphinx tree. Algorithm references and compatibility notes live in `README.md` and `docs/`. |
| [#75 Wrong ellipsoidal quad centroid](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/75) | **Implemented in 0.10.0.** Quad longitude remains the nucleus longitude; latitude is now a one-dimensional Gauss-Legendre integral of the inverse projection. Regression cases cover `Q7`, `O0`, and `P31`. |
| [#71 String prefix/ordering failure for `N_side >= 4`](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/71) | **Already prevented.** Rust identifiers store validated numeric digits in `Vec<u8>` and compare structured identifiers. The public contract currently rejects apertures other than 9 rather than silently misordering them. |
| [#64 Remove matplotlib](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/64) | **Not applicable.** Neither the Rust core nor base Python binding depends on matplotlib or a geometry library. Optional GeoPandas support is isolated behind the `geo` extra. |
| [#62 Rebuilt projection closure per point](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/62) | **Already prevented by architecture.** Projection is compiled Rust code over immutable `Ellipsoid` values; no import, factory, or closure is reconstructed per point. |
| [#60 Cap/ring/antimeridian limitations](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/60) | **Already corrected and tested.** Grid rings traverse cap faces through the edge graph, line coverage handles polar caps, and line/polygon coverage unwraps antimeridian crossings. |
| [#59 Dead code and tracked build artifacts](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/59) | **Not inherited.** The Python stub, old backup outputs, and Sphinx caches do not exist in this repository; Rust/Python build output is ignored. |
| [#58 Packaging, CI, licence, and lint gaps](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/58) | **Already satisfied.** Metadata declares MIT consistently, `Cargo.lock` is tracked, CI tests Linux/Windows/macOS and Python 3.9/3.12/3.14, and rustfmt/clippy are enforced. |
| [#57 Stale Sphinx/README documentation](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/57) | **Not inherited.** The project started with Markdown documentation, current Maturin/Conda setup, and no stale Sphinx toctree or requirements-file references. |
| [#56 Test-coverage gaps](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/56) | **Applicable part implemented in 0.10.0.** A direct authalic transform and bisection inverse now cover strongly flattened custom ellipsoids. Projection, area, hierarchy overlap, error budget, and every boundary shape already have dedicated tests. Visualization-only `color()` remains deliberately excluded. |
| [#55 DE-9IM cell predicates](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/55) | **Implemented in 0.10.0.** Rust and Python expose equality, within/contains, covers/covered-by, touches, disjoint, intersects, crosses, and topological overlaps. Touching supports unequal resolutions and cube seams. Historical object-facade `Cell.overlaps()` keeps its upstream hierarchical meaning; `Cell.topologically_overlaps()` is unambiguous. |
| [#54 Bare `assert` validation](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/54) | **Already prevented.** Public Rust APIs return typed `Result` errors and PyO3 maps invalid values to `ValueError`; optimized Python cannot disable validation. |
| [#53 Mutable shared ellipsoid in neighbours](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/53) | **Already prevented.** `Ellipsoid` is immutable and copied by value. Neighbour calculations never mutate global or shared projection state. |
| [#52 Projection out-of-bounds returns infinity](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/52) | **Already corrected.** Core projection/indexing returns `Error::OutsideProjection`; Python raises `ValueError` or returns `None` only at the facade method whose documented contract is optional cell lookup. |
| [#51 Hard-coded nine-child compaction](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/51) | **Safe within the explicit contract.** Compaction uses the aperture-9 core constant and the package does not advertise `N_side=2`. General aperture support remains an explicit pre-1.0 design decision, so no supported configuration fails silently. |
| [#50 Wrong missing-projection exception](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/50) | **Already prevented.** Only implemented `healpix` and `rhealpix` projections are accepted; unknown names raise `ValueError` directly. There is no dynamic module import. |
| [#49 Quad/cap boundary ignores `n`](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/49) | **Implemented in 0.10.0.** Functional and object APIs return exactly `4*n - 4` points for every shape and honour `interior`. Historical 0.6.0 fixtures remain immutable but tests identify their four-vertex record as a corrected defect. |
| [#48 `boundary(n < 2)` typo](https://github.com/manaakiwhenua/rhealpixdggs-py/issues/48) | **Already corrected with stricter semantics.** Rather than silently clamping bad input, all boundary paths raise a typed error/`ValueError` when fewer than two points per edge are requested. |

## Compatibility decisions

- `Cell.overlaps()` is retained because changing its historical meaning would
  silently break migration code. Use `Cell.topologically_overlaps()` or the
  functional `cell_overlaps()` for the OGC predicate.
- The 0.6.0 conformance corpora are immutable provenance artifacts. Tests keep
  consuming them and explicitly branch only where 0.10.0 fixes a documented
  upstream defect.
- Supporting arbitrary `N_side` values requires a new identifier and encoding
  contract; issue-specific patches do not pretend that capability exists.
