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
