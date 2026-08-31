# Development

## Reproducible environment

The recommended development path installs Rust and the complete native Python
geospatial stack from conda-forge:

```bash
conda env create -f environment-dev.yml
conda activate rhealpix-dev
maturin develop --release
```

This avoids mixing incompatible PROJ, PyProj, GDAL, Pyogrio, and GeoPandas
libraries. On Windows, Visual Studio Build Tools must also provide the
**Desktop development with C++** workload, MSVC x64/x86, and a Windows SDK.

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p rhealpixdggs
cargo check -p rhealpixdggs-python
maturin develop --release
python -m pytest
python tools/generate_readme_figures.py --check
```

The Rust toolchain is pinned in `rust-toolchain.toml`. The Python extension uses
PyO3's Python 3.9 stable ABI.

## README figures

The five SVG figures in `docs/images/` are deterministic outputs of the public
Python API plus a small dependency-free SVG renderer:

```bash
python tools/generate_readme_figures.py
python tools/generate_readme_figures.py --check
```

The generator verifies every documented cell shape, neighbour relationship,
ring, hierarchy path, and integer mapping before writing the assets. CI uses
`--check` to prevent documentation examples drifting from the implementation.

## Conformance corpora

The immutable `tests/fixtures/rhealpixdggs-py-0.6.0` corpora record source
hashes, schemas, checksums, exact identifiers, and floating-point tolerances.
Regeneration requires the exact upstream source tree; commands and provenance
are documented inside that fixture directory. A new upstream contract should
create a new versioned directory rather than modifying an existing corpus.

## Benchmarking

Run `cargo bench -p rhealpixdggs` for Rust microbenchmarks. See
`tools/benchmark_m2.py` and `tools/benchmark_nz_polygon.py --help` for Python
and matched polygon workloads. Performance changes must not silently change
canonical cell identifiers or conformance results.
