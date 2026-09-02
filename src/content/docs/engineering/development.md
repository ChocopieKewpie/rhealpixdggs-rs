---
title: Development
description: Build, test, document, benchmark, and contribute to rhealpixdggs-rs.
---

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
python tools/generate_globe_data.py --check
python tools/build_globe_pmtiles.py --check
python tools/check_documented_python_api.py
npm ci
npm run build
```

The Rust toolchain is pinned in `rust-toolchain.toml`. The Python extension uses
PyO3's Python 3.9 stable ABI.

## README figures

The nine SVG figures in `docs/images/` are deterministic outputs of the public
Python API plus a small dependency-free SVG renderer:

```bash
python tools/generate_readme_figures.py
python tools/generate_readme_figures.py --check
```

The generator verifies every documented cell shape, neighbour relationship,
ring, hierarchy path, and integer mapping before writing the assets. CI uses
`--check` to prevent documentation examples drifting from the implementation.
The cover globe additionally reads the bundled public-domain Natural Earth
1:110m land polygons documented in
`src/content/docs/engineering/natural-earth-data.md`; generation
remains fully offline.

The interactive homepage globe uses the same Natural Earth source to create a
resolution-5 land cover with `polygon_to_cells_intersects()`, compact complete
sibling groups, and export the resulting geographic cell boundaries. Generate
or verify the intermediate GeoJSON, then compile the range-loadable archives:

```bash
python tools/generate_globe_data.py
python tools/generate_globe_data.py --check
python tools/build_globe_pmtiles.py
python tools/build_globe_pmtiles.py --check
```

The tile build uses an installed `tippecanoe`, or a pinned Tippecanoe Node
wrapper when `npx` is available. It creates a compact land/coast archive and a
separate uncompacted-grid archive. The published page range-loads only visible
tiles, never runs polygon coverage in the browser, and does not contact the
larger grid archive until a visitor selects that view. MapLibre itself is
loaded only when the globe is close to the viewport.

The default—and cheapest—production setup is the existing GitHub Pages
deployment. The two `.pmtiles` files are copied from `docs/data/` into the
published site and served from the same origin. No storage account, access
token, CORS configuration, release asset, or repository variable is required.

External object storage is only an optional future scaling path. If the Pages
site eventually outgrows its bandwidth allowance, set these public Astro
environment variables to the absolute URLs of range-enabled storage before the
Astro build:

```bash
PUBLIC_RHEALPIX_PMTILES_URL=https://data.example.org/rhealpix-land-r5.pmtiles
PUBLIC_RHEALPIX_GRID_PMTILES_URL=https://data.example.org/rhealpix-land-r5-grid.pmtiles
npm run build
```

Without these variables—which is the recommended configuration now—local
development and GitHub Pages both use `docs/data/`. The documentation workflow
reads optional GitHub repository variables with the same names but works
normally when they are absent.

## Documentation site

The user guide and API reference are built with Astro Starlight. Node.js 24 is
used in CI:

```bash
npm install
npm run dev
```

Open the URL printed by Astro for local live reload. Run `npm run build` before
submitting documentation changes. The documentation workflow validates pull
requests and deploys `main` to GitHub Pages with Astro's official action.
`tools/check_documented_python_api.py` requires every name exported from the
top-level, NumPy, and GeoPandas modules to have a reference-page heading.

The CAS recipe maps are live-data examples. Regenerate them from the repository
root after building the Python extension and installing the `geo` dependencies:

```bash
python examples/cas_crash_density.py --year 2024 --region "Wellington Region" \
  --resolution 8 --output wellington-cas-r8.gpkg \
  --plot docs/images/wellington-cas-2024-r8.png

python examples/cas_crash_density.py --year 2024 --region "Wellington Region" \
  --resolution 9 --output wellington-cas-r9.gpkg \
  --plot docs/images/wellington-cas-2024-r9.png
```

GeoPackages are ignored by Git; only the finished documentation PNGs are
committed. Counts can change when Waka Kotahi corrects or extends the public
CAS layer.

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
