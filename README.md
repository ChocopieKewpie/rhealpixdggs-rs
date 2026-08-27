# rhealpixdggs-rs

A Rust-first implementation of the aperture-9 rHEALPix Discrete Global Grid
System, with Python as the first supported language binding.

> **Status: early alpha.** Point indexing, projection, shape-aware vertices,
> planar neighbours, hierarchy operations, equal-area metrics, stable integer
> IDs, and an initial upstream object facade are implemented. This is not yet a
> drop-in replacement for every geometry operation in
> [`rhealpixdggs-py`](https://github.com/manaakiwhenua/rhealpixdggs-py).

## Why this layout

The repository has one dependency-free Rust core and thin language bindings:

```text
crates/rhealpixdggs/   Rust projection and indexing core
bindings/python/       PyO3 extension module
python/rhealpixdggs/   Python package and type information
tests/python/          Python API and upstream golden tests
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
children = rh.cell_to_children(cell)
neighbors = rh.cell_to_neighbors(cell)

integer_id = rh.str_to_int(cell)
assert rh.int_to_str(integer_id) == cell
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
```

The facade currently supports aperture 9 on WGS84, configurable polar-square
positions, point indexing, nuclei, vertices, planar neighbours, hierarchy
expansion, and cell metrics. Alternate ellipsoids, alternate apertures, and
ellipsoidal direction names remain on the roadmap.

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
| Shape classification and geographic vertices | Yes | Yes | Golden-tested, including dart trimming |
| Planar edge neighbours | Yes | Yes | Golden-tested, including polar rotations |
| Parent, children, descendants | Yes | Yes | Yes |
| Recursive compact/uncompact | Yes | Yes | Yes |
| String ↔ stable `u64` | Yes | Yes | New API |
| Equal-area cell metric | Yes | Yes | Golden-tested |
| `RHEALPixDGGS` / `Cell` object facade | — | Partial | Core migration calls supported |
| Ellipsoidal neighbour direction names | Planned | Planned | No |
| Lines, polygons, and region filling | Planned | Planned | No |
| Custom aperture / `N_side` | Planned decision | — | No |

See [ROADMAP.md](ROADMAP.md) for the compatibility and performance sequence.

## Development checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test -p rhealpixdggs
cargo check -p rhealpixdggs-python
maturin develop
pytest
cargo bench -p rhealpixdggs
```

The Rust compiler is pinned in `rust-toolchain.toml`. The Python wheel uses
PyO3's Python 3.9 stable ABI so one wheel can serve multiple CPython versions
on each platform.

## Compatibility policy

The first target is numerical and identifier parity with the upstream
`WGS84_003` configuration. Upstream outputs are committed as golden tests. New
algorithms should be benchmarked against both the Rust implementation and the
released Python package; speed changes must not silently change cell IDs.

The Python surface is intentionally H3-like for new code. The separate
`RHEALPixDGGS` and `Cell` facade preserves upstream coordinate ordering and is
being expanded incrementally as matching core semantics land.

## Licence and attribution

MIT. Projection and indexing mathematics were ported from
`manaakiwhenua/rhealpixdggs-py` under its MIT licence option. See
[LICENSE](LICENSE) and [NOTICE](NOTICE).
