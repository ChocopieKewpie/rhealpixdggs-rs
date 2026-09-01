---
title: Upstream compatibility boundary
description: See which rhealpixdggs-py behaviours are preserved and where APIs differ.
---

M1 targets the deterministic ordering, traversal, projection, cell geometry,
and coverage behavior of `rHEALPixDGGS` 0.6.0 for the upstream `WGS84_003`
configuration. Python keeps upstream's `(longitude, latitude)` coordinate order
inside the `RHEALPixDGGS` and `Cell` facade; the H3-like functional API keeps
its `(latitude, longitude)` convention.

## Deterministic object facade

| Upstream class | Supported M1 methods |
|---|---|
| `RHEALPixDGGS` projection | `healpix`, `rhealpix`, `combine_triangles`, `triangle`, `xyz`, `xyz_cube` |
| `RHEALPixDGGS` hierarchy | `cell`, `grid`, `num_cells`, `interval` |
| `RHEALPixDGGS` indexing and coverage | `cell_from_point`, `cell_from_region`, `cell_latitudes`, `cells_from_meridian`, `cells_from_parallel`, `cells_from_line`, `cells_from_region`, `minimal_cover`, `antimeridian_check_and_flip` |
| `RHEALPixDGGS` metrics | `cell_width`, `cell_area`, `area_error_budget` |
| `Cell` ordering and traversal | comparisons, `index`, `suid_rowcol`, `successor`, `predecessor`, `subcell`, `subcells`, `rotate_entry`, `rotate` |
| `Cell` geometry | `width`, `area`, `ul_vertex`, `nw_vertex`, `nucleus`, `vertices`, `xy_range`, `boundary`, `interior`, point/cell `contains`, `intersects_meridian`, `intersects_parallel`, `overlaps`, `region_overlaps`, `region`, `ellipsoidal_shape`, `centroid` |
| `Cell` topology | `neighbor`, `neighbors`, `equals`, `within`, `covers`, `covered_by`, `touches`, `disjoint`, `intersects`, `crosses`, `topologically_overlaps` |

The facade is backed by Rust for projection, indexing, vertices, boundaries,
centroids, neighbors, coverage, metrics, and traversal indices. Small object
composition operations remain in Python so the public shape matches upstream
without leaking Python types into the core crate.

## Explicitly outside M1

- custom ellipsoids, radians-mode ellipsoids, and `N_side` values other than 3;
- nondeterministic `random_point`, `random_cell`, and `Cell.random_point` calls;
- visualization-only `Cell.color` and geometry-library helper classes;
- exact reproduction of diagnostic `__str__` output and empty/invalid upstream
  `Cell` objects.

These exclusions do not affect canonical cell IDs or deterministic WGS84_003
geometry. Alternate grid configurations remain an open design decision; random
sampling and visualization adapters can be built separately without becoming
part of the cross-language core contract.

## Conformance evidence

Three immutable, versioned corpora under
`tests/fixtures/rhealpixdggs-py-0.6.0` define the M1 contract:

- `conformance-v1.json`: 1,583 point, geometry, topology, and metric cases;
- `coverage-v1.json`: 16 exact-edge, region, line, and polygon cases;
- `facade-v1.json`: 63 projection, region-parent, latitude traversal,
  meridian/parallel, interval, identifier, predicate, and Cartesian cases.

The Rust integration tests and Python tests both consume these fixtures. The
projected absolute error budget is 20 nanometres for the facade fixture and
the geographic budget is `2e-10` degrees. Existing broader geometry fixtures
retain their documented tolerances. Antimeridian and polar corrections that
upstream itself documents as unsupported are tested separately and are treated
as intentional fixes rather than regressions.

The corrected contract also fixes two upstream 0.6.0 defects: quad/cap
boundaries honour `n` and `interior`, and quad centroids integrate nonlinear
latitude rather than averaging edge latitudes. The immutable corpus remains
historical evidence; tests explicitly distinguish corrected behavior where an
old fixture records one of these defects.

The upstream object method `Cell.overlaps()` means hierarchical containment in
either direction, so it remains available for migration compatibility. That is
not the OGC DE-9IM overlaps predicate. New code can use the unambiguous
`Cell.topologically_overlaps()` method or functional `cell_overlaps()` call;
both correctly return false for cells in one nested DGGS hierarchy.
