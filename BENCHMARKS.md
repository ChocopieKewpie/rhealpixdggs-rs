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
| Polar shape classification at resolution 12 | 5.3 ns |
| Four polar geographic vertices at resolution 12 | 533 ns |

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

The upstream optional dependency imports were replaced with inert stubs during
this isolated comparison; imports and object construction occurred outside the
timed loop, and the measured `cell_from_point` path still used upstream's own
Python rHEALPix projection and cell-selection code.
