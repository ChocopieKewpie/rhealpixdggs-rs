---
title: Installation
description: Install the Python extension, Rust crate, or reproducible development environment.
---

The Python package contains a native Rust extension. A prebuilt wheel is the
simplest installation route; source and development installs also require Rust
and a platform linker.

## Python wheel

Install a wheel built for your Python version and operating system:

```bash
python -m pip install rhealpixdggs_rs-0.10.1-<platform>.whl
```

When a release is published to PyPI, the equivalent command is:

```bash
python -m pip install rhealpixdggs-rs
```

Verify the extension:

```bash
python -c "import rhealpixdggs as rh; print(rh.__version__)"
```

## Geospatial extras

The Rust core and standard Python API only require NumPy. GeoPandas, Shapely,
Pyogrio, GDAL, and PROJ are optional:

```bash
python -m pip install "rhealpixdggs-rs[geo]"
```

For Conda-based GIS development, use the checked-in environment. It installs
Rust inside the environment as well as compatible PROJ/GDAL packages:

```bash
conda env create -f environment-dev.yml
conda activate rhealpix-dev
maturin develop --release
python -m pytest
```

## Build from source

```bash
git clone https://github.com/ChocopieKewpie/rhealpixdggs-rs.git
cd rhealpixdggs-rs
python -m venv .venv
```

=== "Windows (PowerShell)"

    ```powershell
    .\.venv\Scripts\Activate.ps1
    python -m pip install --upgrade pip maturin pytest numpy
    maturin develop --release
    python -m pytest
    ```

=== "Linux / macOS"

    ```bash
    source .venv/bin/activate
    python -m pip install --upgrade pip maturin pytest numpy
    maturin develop --release
    python -m pytest
    ```

On Windows, install Visual Studio Build Tools with **Desktop development with
C++**, including MSVC x64/x86 and a Windows SDK. VS Code does not install
`link.exe`.

## Documentation site

```bash
npm install
npm run dev
```

Open the local URL printed by Astro. Use `npm run build` before submitting
documentation changes.
