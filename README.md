# rhealpixdggs-rs

A Rust-first implementation of the aperture-9 rHEALPix Discrete Global Grid
System, with Python as the first supported language binding.

> **Status: pre-1.0; version 0.8.1 includes completed M1 semantic parity, the
> M2 bulk API, robust small-polygon validation, and optional GeoPackage polygon
> conversion.** Point indexing,
> projection, shape-aware vertices,
> exact densified boundaries, planar and ellipsoidal neighbours, hierarchy
> traversal and ordering, equal-area metrics, stable integer IDs, dependency-
> free rectangle/line/polygon coverage, and the deterministic `WGS84_003`
> upstream object facade are implemented. Custom ellipsoids, alternate
> apertures, random sampling, and visualization helpers are not yet drop-in
> compatible with
> [`rhealpixdggs-py`](https://github.com/manaakiwhenua/rhealpixdggs-py).

## Why this layout

The repository has one dependency-free Rust core and thin language bindings:

```text
crates/rhealpixdggs/   Rust projection and indexing core
bindings/python/       PyO3 extension module
python/rhealpixdggs/   Python package and type information
tests/python/          Python API and upstream golden tests
tests/fixtures/        Versioned cross-language conformance corpora
benchmarks/data/       Reproducible non-authoritative benchmark fixtures
tools/                 Deterministic corpus and development utilities
```

That avoids reimplementing rHEALPix independently in each language. The cell
ID string (`R88446545`) and stable `u64` representation form the shared
interchange contract for future C, JavaScript/Wasm, Java, and R bindings.

## Python quick start

```bash
conda env create -f environment-dev.yml
conda activate rhealpix-dev
maturin develop --release
pytest
```

The development environment installs Python, Rust/Cargo, Maturin, and the
geospatial benchmark stack from `conda-forge`. Keeping PROJ, PyProj, GDAL,
Pyogrio, and GeoPandas in the same Conda environment avoids incompatible
native libraries. On Windows, first install the Visual Studio Build Tools
**Desktop development with C++** workload (MSVC x64/x86 and a Windows SDK);
VS Code alone does not provide `link.exe`.

```python
import rhealpixdggs as rh

cell = rh.latlng_to_cell(-40.356, 175.611, 12)
assert cell == "R887560473610"

lat, lng = rh.cell_to_latlng(cell)
boundary = rh.cell_to_boundary(cell)
dense_boundary = rh.cell_to_boundary_densified(cell, points_per_edge=16)
assert len(dense_boundary) == 60  # exactly 4 * 16 - 4
children = rh.cell_to_children(cell)
neighbors = rh.cell_to_neighbors(cell)
geographic_neighbors = rh.cell_to_neighbors(cell, plane=False)

line_cells = rh.line_to_cells([(-40.356, 175.611), (-40.35, 175.62)], 12)
polygon_cells = rh.polygon_to_cells(
    [(-40.36, 175.60), (-40.34, 175.60), (-40.34, 175.63), (-40.36, 175.63)],
    12,
    compact=True,
)

integer_id = rh.str_to_int(cell)
assert rh.int_to_str(integer_id) == cell

post_index = rh.cell_to_post_order_index(cell)
assert rh.post_order_index_to_cell(post_index) == cell
next_cell = rh.cell_to_successor(cell)
```

The Python API follows H3's coordinate convention: functions accept and return
`(latitude, longitude)`. The Rust core uses `(longitude, latitude)`.

Large workloads can use one extension call per contiguous NumPy batch. Integer
cell IDs avoid constructing a Python string for every point; returned arrays
are read-only zero-copy views over the extension's immutable result bytes:

```python
import numpy as np
from rhealpixdggs import numpy as rhnp

points = np.array([[-40.356, 175.611], [37.7749, -122.4194]])
cells = rhnp.latlngs_to_cells(points, 12)
nuclei = rhnp.cells_to_latlngs(cells)
boundaries = rhnp.cells_to_boundaries(cells, points_per_edge=16)

# Ragged batch output uses CSR-style offsets.
covered, offsets = rhnp.bboxes_to_cells(
    [[-40.0, -42.0, 176.0, 173.0], [11.0, 9.0, -178.0, 178.0]], 8
)
second_box = covered[offsets[1] : offsets[2]]
```

Bulk computation releases the GIL. `parallel=None` automatically enables
Rayon above benchmarked crossover sizes; pass `parallel=True` or `False` to
override that policy.

Install the optional geospatial adapter to convert Polygon/MultiPolygon vector
data directly to an EPSG:4326 GeoPackage:

```bash
python -m pip install -e '.[geo]'
rhealpix-to-gpkg nz-boundary.gpkg nz-rhealpix.gpkg --resolution 6
```

```python
from rhealpixdggs.geo import polygon_file_to_geopackage

cells = polygon_file_to_geopackage(
    "nz-boundary.gpkg",
    "nz-rhealpix.gpkg",
    resolution=6,
)
print(f"wrote {len(cells):,} cells")
```

Inputs may use any declared CRS; they are reprojected to WGS84 and dissolved
before coverage. Output cells include `cell_id`, `resolution`, `area_m2`, and
antimeridian-safe MultiPolygon geometry. Existing outputs are protected unless
the caller explicitly passes `overwrite=True` or `--overwrite`.

Existing `rhealpixdggs-py` code can start migrating through the object facade,
which retains the upstream `(longitude, latitude)` convention:

```python
from rhealpixdggs import WGS84_003

cell = WGS84_003.cell(("N", 6, 2))
assert cell.ellipsoidal_shape == "dart"
assert str(cell.neighbor("up")) == "N38"
vertices = cell.vertices(plane=False, trim_dart=True)
boundary = cell.boundary(n=16, plane=False)
assert WGS84_003.cell(post_order_index=cell.index("post")) == cell
next_cell = cell.successor()
region = WGS84_003.cells_from_region(
    3, (170.0, -35.0), (176.0, -42.0), plane=False
)
```

The facade supports aperture 9 on WGS84, configurable polar-square positions,
projection helpers, point and region indexing, nuclei and centroids, vertices,
planar and ellipsoidal neighbours, boundaries and interiors, predicates,
ordering and traversal, hierarchy expansion, meridian/parallel/line/region
coverage, and cell metrics. See
[`docs/UPSTREAM_COMPATIBILITY.md`](docs/UPSTREAM_COMPATIBILITY.md) for the exact
M1 boundary and deliberate exclusions.

The new functional `cell_to_boundary_densified` call has an exact contract for
every shape: `points_per_edge >= 2` and `4 * points_per_edge - 4` returned
points, ordered clockwise from geographic northwest. For migration parity,
`Cell.boundary()` retains upstream's geographic shortcut: quad and cap cells
return four vertices, while dart and skew-quad cells use the requested density.

## Initial performance

On the development benchmark machine, a warmed Python point-to-cell call took
0.485 µs here versus 20.285 µs in `rhealpixdggs-py` 0.6.0—about **41.8× faster**
for that specific single-point operation. Rust-core point indexing measured
about 170 ns. See [BENCHMARKS.md](BENCHMARKS.md) for the setup and important
limits of this early comparison, including the recorded Windows M2 run and the
reproducible New Zealand polygon workload.

On the matched Windows New Zealand workload, both implementations selected the
same 1,859 resolution-6 cells. Rust 0.8.0 completed polygon coverage in
14.582 ms versus 72.535 s upstream (**4,974× faster**) and the full GeoPackage
pipeline in 99.991 ms versus 72.414 s (**724× faster**). These values are one
measured sample after warm-up because repeated legacy runs were impractical.

## Rust quick start

```rust
use rhealpixdggs::RhealpixDggs;

let dggs = RhealpixDggs::wgs84_003();
let cell = dggs.cell_from_lonlat(175.611, -40.356, 12)?;
assert_eq!(cell.to_string(), "R887560473610");

let (longitude, latitude) = dggs.cell_to_lonlat(&cell)?;
# Ok::<(), rhealpixdggs::Error>(())
```

## Implemented API

| Capability | Rust | Python | Upstream parity |
|---|---:|---:|---:|
| WGS84_003 point → cell | Yes | Yes | Golden-tested |
| NumPy point/cell conversion | Ordered bulk | Yes | Scalar-equivalent |
| Batch boundaries and bounding boxes | Ordered bulk | Yes | Scalar-equivalent |
| Cell → projected/geographic nucleus | Yes | Yes | Golden-tested |
| Ellipsoidal cell centroid | Yes | Yes | Upstream-compatible quadrature |
| Shape classification and geographic vertices | Yes | Yes | Golden-tested, including dart trimming |
| Exact densified projected/geographic boundaries | Yes | Yes | Differential-tested across shapes and polar layouts |
| Planar edge neighbours | Yes | Yes | Golden-tested, including polar rotations |
| Ellipsoidal edge neighbours | Yes | Yes | Exhaustively differential-tested through resolution 3 |
| Parent, children, descendants | Yes | Yes | Yes |
| Post-order comparison and predecessor/successor traversal | Yes | Yes | Exhaustively differential-tested through resolution 3 |
| Level/post-order index ↔ cell | Yes | Yes | Differential-tested; two upstream defects corrected |
| Recursive compact/uncompact | Yes | Yes | Yes |
| String ↔ stable level-order `u64` | Yes | Yes | New API |
| Equal-area cell metric | Yes | Yes | Golden-tested |
| Versioned upstream conformance corpus | Yes | Yes | 1,583 shared cases from upstream 0.6.0 |
| Rectangle, polyline, and polygon coverage | Yes | Yes | Shared upstream corpus plus antimeridian corrections |
| Polygon vector input → GeoPackage cells | Core coverage | Optional `geo` extra | Same centroid-selection contract; benchmarked separately |
| `RHEALPixDGGS` / `Cell` deterministic object facade | Core-backed | Yes | M1 complete for documented WGS84_003 boundary |
| Custom aperture / `N_side` | Planned decision | — | No |

See [ROADMAP.md](ROADMAP.md) for the compatibility and performance sequence.

## Development checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test -p rhealpixdggs
cargo check -p rhealpixdggs-python
python tools/generate_upstream_coverage_corpus.py \
  --upstream-root ../upstream-src --check
python tools/generate_upstream_facade_corpus.py \
  --upstream-root ../upstream-src --check
maturin develop --release
pytest
cargo bench -p rhealpixdggs
python tools/benchmark_nz_polygon.py --help
```

The Rust compiler is pinned in `rust-toolchain.toml`. The Python wheel uses
PyO3's Python 3.9 stable ABI so one wheel can serve multiple CPython versions
on each platform.

## Compatibility policy

The completed M1 target is numerical and identifier parity with the upstream
`WGS84_003` configuration. Upstream 0.6.0 outputs are committed as a versioned,
language-neutral conformance corpus consumed by both the Rust core and Python
binding. The base corpus covers all 16 polar-square layouts, 1,344
point-indexing cases, 208 geometry cases, traversal and ordering, and metrics
through resolution 15; versioned coverage and facade corpora extend it. See
[`tests/fixtures/rhealpixdggs-py-0.6.0`](tests/fixtures/rhealpixdggs-py-0.6.0)
for its provenance, schema, checksum, and deterministic regeneration command.
New algorithms should be benchmarked against both the Rust implementation and
the released Python package; speed changes must not silently change cell IDs.

The Python surface is intentionally H3-like for new code. The separate
`RHEALPixDGGS` and `Cell` facade preserves upstream coordinate ordering for the
documented deterministic M1 surface.

Known upstream defects are corrected rather than reproduced: resolution-zero
level indices are `0..=5` and round-trip correctly, and successor traversal
past terminal cell `S` returns `None` at finer resolutions instead of raising
an internal `AttributeError`. Coverage also unwraps antimeridian-crossing lines
and polygons and explicitly handles polar cap cells, both known limitations in
the upstream line implementation.

## Licence and attribution

MIT. The Rust implementation is maintained by James Ardo and contributors.
Projection and indexing mathematics were ported from
`manaakiwhenua/rhealpixdggs-py` under its MIT licence option. See
[LICENSE](LICENSE), [NOTICE](NOTICE), and
[`docs/NUMERICAL_ACCURACY.md`](docs/NUMERICAL_ACCURACY.md). The optimized
authalic-latitude series follows the method evaluated by Gilić and Gašparović
(2025), DOI [10.1109/JSTARS.2025.3567839](https://doi.org/10.1109/JSTARS.2025.3567839).
