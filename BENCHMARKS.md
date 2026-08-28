# Initial performance baseline

These are development baselines, not universal performance guarantees.

Measured on 2026-08-27 in a Linux x86_64 container on an AMD EPYC 9V74,
Rust 1.85.0, CPython 3.12, release profile. Criterion used its default warm-up
and 100 samples. The test point was `(longitude=175.611, latitude=-40.356)` at
resolution 12.

## Rust core

| Operation | Median estimate |
|---|---:|
| Longitude/latitude → cell | 170 ns |
| Cell → longitude/latitude nucleus | 145 ns |
| Parse and reformat a resolution-12 string ID | 280 ns |
| Polar planar neighbour at resolution 12 | 60 ns |
| Four ellipsoidal neighbours for a polar cell at resolution 12 | 1.07 µs |
| Polar shape classification at resolution 12 | 5.3 ns |
| Four polar geographic vertices at resolution 12 | 533 ns |
| 60-point projected boundary at resolution 12 | 98.5 ns |
| 60-point geographic polar boundary at resolution 12 | 6.44 µs |
| Post-order index at resolution 12 | 34.3 ns |
| Cell from post-order index at resolution 12 | 79.7 ns |
| Successor at requested resolution 15 | 43.9 ns |

Reproduce with:

```bash
cargo bench -p rhealpixdggs --bench indexing
```

## Python single-call comparison

The same point-to-string-cell operation was timed after warm-up:

| Implementation | Time/call | Calls/second |
|---|---:|---:|
| This PyO3 binding | 0.485 µs | 2,062,267 |
| `rhealpixdggs-py` 0.6.0 | 20.285 µs | 49,298 |

That is approximately **41.8× faster** for this specific single-point path.
The comparison includes Python call and string-return overhead in both cases.
It does not measure polygon coverage, boundaries, NumPy/Arrow batches, or
multi-threading. Those need separate benchmarks as they are implemented.

The upstream-compatible Python `Cell.boundary(n=16, plane=False)` call was
also measured on the `N0` dart cell, returning 60 points in both libraries:

| Implementation | Time/call | Relative |
|---|---:|---:|
| This PyO3 binding | 8.481 µs | 1× |
| `rhealpixdggs-py` 0.6.0 | 2,963.057 µs | 349.4× slower |

This boundary comparison used 200 calls per repeat, five repeats after warm-up,
and reports the best repeat. It exercises inverse projection and Python object
creation but is still one cell shape, density, machine, and build; it is not a
general performance guarantee.

Boundary correctness was separately compared with upstream over every cell
through resolution 2 in four polar-square configurations, at 2, 3, and 5
points per edge, with planar/geographic and inset/non-inset modes. All 26,208
boundary calls matched within 0.2 µm projected or `2e-10` degrees geographic.

For a resolution-12 `Cell.index("post")` facade call, this binding measured
0.251 µs versus 4.524 µs upstream, or **18.0× faster**. The equivalent Rust
operation is the 34.3 ns core result above; the remaining time is Python call
and object overhead.

Ordering and traversal were differentially checked across all 4,920 cells
through resolution 3: 39,348 predecessor/successor calls, all level/post index
round trips, and a 546-cell mixed-resolution sort matched upstream. Ten cases
were intentional corrections: upstream offsets the six resolution-zero level
indices by six and crashes when requesting a finer successor after terminal
cell `S`; this implementation returns indices `0..=5` and `None`, respectively.

The upstream optional dependency imports were replaced with inert stubs during
this isolated comparison; imports and object construction occurred outside the
timed loop, and the measured `cell_from_point` path still used upstream's own
Python rHEALPix projection and cell-selection code.

## M2 bulk baseline

Measured on 2026-08-28 in the same Linux x86_64 environment, now with 9 logical
CPUs available, CPython 3.12.13, NumPy 2.5.2, and a release build. Values below
are medians of five warmed Python samples. The upstream rows execute the
equivalent 0.6.0 scalar operation once per point because that release has no
array API. The NumPy rows include normalization, one contiguous input-buffer
copy, Rust computation, and creation of a read-only zero-copy output view.

| Batch | Implementation/mode | Median latency | Throughput | Traced peak memory |
|---:|---|---:|---:|---:|
| 256 | Rust NumPy automatic | 51.3 µs | 4.99 M points/s | 6.2 KiB |
| 256 | Upstream Python loop | 5.16 ms | 49.6 k points/s | 16.2 KiB |
| 4,096 | Rust NumPy automatic | 1.21 ms | 3.37 M points/s | 96.1 KiB |
| 4,096 | Upstream Python loop | 93.3 ms | 43.9 k points/s | 237.6 KiB |
| 65,536 | Rust NumPy automatic | 6.50 ms | 10.1 M points/s | 1.50 MiB |
| 65,536 | Upstream Python loop | 1.384 s | 47.4 k points/s | 3.73 MiB |

At 65,536 points the complete NumPy call was about **213× faster** and its
traced peak was about **2.5× smaller** than the upstream string-producing loop.
The fixed result payload itself is 8 bytes per cell ID; the input payload is 16
bytes per latitude/longitude pair. Results use the stable integer encoding and
can be converted to strings only where a text boundary requires it.

A 4,096-cell geographic boundary batch with four points per edge (12 points
per cell) took a 2.03 ms median, or about 2.02 million cells / 24.3 million
boundary points per second. Its output payload was 768 KiB and traced peak was
800 KiB.

### Parallel crossover

Criterion used one-second warm-up, three-second measurement, and 20 samples.
Medians shown are sequential versus forced Rayon execution:

| Operation | Batch | Sequential | Rayon | Selected automatic threshold |
|---|---:|---:|---:|---:|
| Longitude/latitude → cell | 256 | 43.3 µs | 183.0 µs | 4,096 |
| Longitude/latitude → cell | 4,096 | 982.1 µs | 557.3 µs | 4,096 |
| Cell → nucleus | 256 | 30.5 µs | 177.9 µs | 4,096 |
| Cell → nucleus | 4,096 | 522.3 µs | 374.3 µs | 4,096 |
| 12-point boundary | 64 | 103.6 µs | 127.3 µs | 512 |
| 12-point boundary | 512 | 669.8 µs | 341.3 µs | 512 |
| Small region cover | 64 | 122.0 µs | 143.7 µs | 256 |
| Small region cover | 256 | 321.2 µs | 228.9 µs | 256 |

Parallel measurements are noisier on shared CI/container hardware, especially
when callers force Rayon below the crossover. Automatic mode uses the table's
conservative thresholds. Explicit `parallel=True` and `parallel=False` remain
available for workload-specific tuning.

Reproduce the core measurements with:

```bash
cargo bench -p rhealpixdggs --features parallel --bench bulk
```

Reproduce the Python and upstream comparison from an unpacked 0.6.0 source
tree with:

```bash
python tools/benchmark_m2.py \
  --mode all \
  --upstream-root ../upstream-src
```

The benchmark emits versioned JSON to standard output, including OS, Python,
CPU-count, latency ranges, throughput, traced peak memory, and active threshold.
The manual `M2 Benchmarks` GitHub Actions workflow runs the current
implementation on Linux, Windows, and macOS and retains both Python JSON and
Criterion output as build artifacts.

## Windows x86_64 M2 record

Measured on 2026-08-28 on Windows with an Intel Core i5-14400F (10 cores, 16
logical processors), CPython 3.12.14, Rust 1.98.0, and the MSVC target. The
source was the completed M2 benchmark package at commit `23133cb`; its package
metadata still reported `0.1.0-alpha.1` before the 0.8.0 version bump. Values
below are medians of five warmed calls.

| Batch | Mode | Median latency | Throughput | Traced peak memory |
|---:|---|---:|---:|---:|
| 256 | Automatic | 45.2 µs | 5.66 M points/s | 6.1 KiB |
| 4,096 | Automatic | 1.012 ms | 4.05 M points/s | 96.1 KiB |
| 65,536 | Sequential | 13.719 ms | 4.78 M points/s | 1.50 MiB |
| 65,536 | Automatic | 5.023 ms | 13.0 M points/s | 1.50 MiB |
| 65,536 | Forced Rayon | 4.817 ms | 13.6 M points/s | 1.50 MiB |

The automatic 65,536-point path was about **2.73× faster** than sequential on
this machine. A 4,096-cell, 12-point geographic boundary batch took 1.775 ms.
The independent Criterion run measured 21.38 million point conversions/s for
forced Rayon versus 5.59 million/s sequential at 65,536 elements, and 5.19
million versus 1.06 million cell boundaries/s at 4,096 elements. Differences
between the Python and Criterion ratios are expected because the Python result
includes array normalization and the extension boundary.

Linux x86_64 and Windows x86_64 are now recorded. The macOS arm64 measurement
is **TBD (hardware unavailable)**; it is a deferred portability record rather
than a blocker for subsequent milestones.

## New Zealand polygon → GeoPackage benchmark

Version 0.8.0 adds two vector workloads that time input reading,
centroid-based polygon coverage, cell-boundary generation, GeoPackage writing,
and the complete pipeline separately:

1. The default cross-implementation workload uses the deterministic
   `new-zealand-simplified.geojson` fixture at resolution 6. It is intentionally
   small enough for upstream Python 0.6.0 to complete.
2. The Rust-scale workload uses the 17-part, 4,149-vertex
   `new-zealand.geojson` fixture at resolution 8. It comes from SimpleMaps under
   CC BY 4.0 and is not an authoritative national boundary. Upstream 0.6.0 is
   not used for this tier because it is impractically slow.

The current adapter emits exactly `4 * points_per_edge - 4` vertices for every
cell. The 0.6.0 `Cell.boundary` path shortcuts quad and cap cells to four
vertices, so the report records each boundary contract alongside the timings;
the cell-set comparison remains exact and independent of output densification.

Run the default comparison workload with the current implementation:

```bash
python tools/benchmark_nz_polygon.py \
  --mode current \
  --input benchmarks/data/new-zealand-simplified.geojson \
  --resolution 6 \
  --output-dir benchmark-results
```

For a direct comparison, install upstream 0.6.0 in a separate environment
because both distributions use the `rhealpixdggs` import name. Run the driver
from the 0.8.0 environment and point it at the legacy interpreter:

```bash
python tools/benchmark_nz_polygon.py \
  --mode all \
  --legacy-python /path/to/legacy/environment/bin/python \
  --input benchmarks/data/new-zealand-simplified.geojson \
  --resolution 6 \
  --output-dir benchmark-results
```

Progress is written to standard error, so it remains visible when JSON standard
output is redirected to a file. The detailed Rust-only tier is:

```bash
python tools/benchmark_nz_polygon.py \
  --mode current \
  --input benchmarks/data/new-zealand.geojson \
  --resolution 8 \
  --output-dir benchmark-results-r8
```

### Recorded Windows comparison

The matched Windows run supplied on 2026-08-28 used CPython 3.12.14, one
measured sample after warm-up, resolution 6, four points per edge, and 1,859
output cells. Rust 0.8.0 and upstream Python 0.6.0 produced the same cell-ID
checksum (`66d5bfd…f5478`).

| Stage | Rust 0.8.0 | Python 0.6.0 | Speedup |
|---|---:|---:|---:|
| Input read | 1.592 ms | 1.954 ms | 1.23× |
| Polygon coverage | 14.582 ms | 72.535 s | **4,974.46×** |
| Cell boundaries | 67.014 ms | 637.669 ms | **9.52×** |
| GeoPackage write | 21.401 ms | 29.265 ms | 1.37× |
| End-to-end | 99.991 ms | 72.414 s | **724.20×** |

The coverage stage's traced Python peak was 255 KiB for Rust and 1.39 MiB for
legacy, a 5.60× ratio. Treat the exact latency ratios as indicative because
the legacy cost made repeated sampling impractical; the several-orders-of-
magnitude coverage difference is nevertheless unambiguous. The normalized raw
record is in
`benchmarks/results/nz-simple-r6-windows-x86_64.json`.

The JSON report includes exact cell-set checksums and reports any differing
cells before calculating speedups. It writes
`new-zealand-rhealpix-rust.gpkg` and
`new-zealand-rhealpix-legacy.gpkg` for visual inspection. Python
`tracemalloc` values cover Python allocations, not all memory allocated inside
GEOS, GDAL, NumPy, or Rust.
