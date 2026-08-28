# Contributing

## Local setup

Use the repository's Conda environment so Python, Rust/Cargo, Maturin, and the
native geospatial dependencies come from one `conda-forge` toolchain:

```bash
conda env create -f environment-dev.yml
conda activate rhealpix-dev
maturin develop --release

cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test -p rhealpixdggs
cargo check -p rhealpixdggs-python
pytest
```

On Windows, install the Visual Studio Build Tools **Desktop development with
C++** workload, including MSVC x64/x86 and a Windows SDK, before creating the
environment. The Conda `rust` package supplies `rustc` and Cargo, while the
Microsoft workload supplies the required `link.exe` system linker.

## Compatibility changes

Any change to projection or cell-selection logic must add or update an upstream
golden fixture and explain numerical differences. Do not update expected cell
IDs merely to make a failing test pass.

## Performance changes

Include a Criterion benchmark or a Python benchmark that measures the affected
path. Report input size, platform, warm-up, and whether Python conversion time
is included.

## Commits and pull requests

Keep the core independent of binding-specific types. Public API changes should
update the Python stubs, README compatibility table, and roadmap as applicable.
