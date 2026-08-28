# rhealpixdggs-rs

A Rust-first implementation of the aperture-9 rHEALPix Discrete Global Grid
System, with Python as the first supported language binding.

> **Status: early alpha.** Point indexing, projection, shape-aware vertices,
> exact densified boundaries, planar and ellipsoidal neighbours, hierarchy
> traversal and ordering, equal-area metrics, stable integer IDs, dependency-
> free rectangle/line/polygon coverage, and an initial upstream object facade
> are implemented.
> This is not yet a drop-in replacement for every geometry operation in
> [`rhealpixdggs-py`](https://github.com/manaakiwhenua/rhealpixdggs-py).

## Why this layout

The repository has one dependency-free Rust core and thin language bindings:

```text
crates/rhealpixdggs/   Rust projection and indexing core
bindings/python/       PyO3 extension module
python/rhealpixdggs/   Python package and type information
tests/python/          Python API and upstream golden tests
tests/fixtures/        Versioned cross-language conformance corpora
tools/                 Deterministic corpus and development utilities
```

That avoids reimplementing rHEALPix independently in each language. The cell
ID string (`R88446545`) and stable `u64` representation form the shared
interchange contract for future C, JavaScript/Wasm, Java, and R bindings.

## Python quick start

```bash
python -m venv .venv
source .venv/bin/activate       # Windows: .venv\Scripts\activate
python -m pip install maturin pytest
maturin develop
pytest
```

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

The facade currently supports aperture 9 on WGS84, configurable polar-square
positions, point indexing, nuclei, vertices, planar and ellipsoidal neighbours,
densified boundaries, ordering and traversal, hierarchy expansion, and cell
metrics. Region and line coverage are also available through the facade.
Alternate ellipsoids and alternate apertures remain on the roadmap.

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
limits of this early comparison.

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
| `RHEALPixDGGS` / `Cell` object facade | — | Partial | Includes upstream boundary semantics |
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
maturin develop
pytest
cargo bench -p rhealpixdggs
```

The Rust compiler is pinned in `rust-toolchain.toml`. The Python wheel uses
PyO3's Python 3.9 stable ABI so one wheel can serve multiple CPython versions
on each platform.

## Compatibility policy

The first target is numerical and identifier parity with the upstream
`WGS84_003` configuration. Upstream 0.6.0 outputs are committed as a versioned,
language-neutral conformance corpus consumed by both the Rust core and Python
binding. It covers all 16 polar-square layouts, 1,344 point-indexing cases, 208
geometry cases, traversal and ordering, and metrics through resolution 15. See
[`tests/fixtures/rhealpixdggs-py-0.6.0`](tests/fixtures/rhealpixdggs-py-0.6.0)
for its provenance, schema, checksum, and deterministic regeneration command.
New algorithms should be benchmarked against both the Rust implementation and
the released Python package; speed changes must not silently change cell IDs.

The Python surface is intentionally H3-like for new code. The separate
`RHEALPixDGGS` and `Cell` facade preserves upstream coordinate ordering and is
being expanded incrementally as matching core semantics land.

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
[LICENSE](LICENSE) and [NOTICE](NOTICE).
