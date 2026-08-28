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
