#!/usr/bin/env python3
"""Reproducible Python M2 benchmark against rhealpixdggs-py 0.6.0."""

from __future__ import annotations

import argparse
import json
import os
import platform
import statistics
import subprocess
import sys
import time
import tracemalloc
import types
from pathlib import Path
from typing import Any, Callable


def coordinates(count: int) -> list[tuple[float, float]]:
    return [
        (
            (index * 0.414_213_562_373_095) % 178.0 - 89.0,
            (index * 0.618_033_988_749_895) % 358.0 - 179.0,
        )
        for index in range(count)
    ]


def measurement(call: Callable[[], Any], repeats: int) -> dict[str, float | int]:
    call()
    samples = []
    for _ in range(repeats):
        started = time.perf_counter_ns()
        result = call()
        samples.append(time.perf_counter_ns() - started)
        del result
    tracemalloc.start()
    result = call()
    peak = tracemalloc.get_traced_memory()[1]
    tracemalloc.stop()
    del result
    median = statistics.median(samples)
    return {
        "median_ns": int(median),
        "min_ns": min(samples),
        "max_ns": max(samples),
        "traced_peak_bytes": peak,
    }


def scalar_measurement(
    call: Callable[[], Any], repeats: int, iterations: int = 1_000
) -> dict[str, float | int]:
    call()
    samples = []
    for _ in range(repeats):
        started = time.perf_counter_ns()
        for _ in range(iterations):
            result = call()
        samples.append((time.perf_counter_ns() - started) // iterations)
        del result
    tracemalloc.start()
    result = call()
    peak = tracemalloc.get_traced_memory()[1]
    tracemalloc.stop()
    del result
    return {
        "median_ns": int(statistics.median(samples)),
        "min_ns": min(samples),
        "max_ns": max(samples),
        "traced_peak_bytes": peak,
    }


def environment() -> dict[str, str | int]:
    return {
        "os": platform.platform(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "logical_cpus": os.cpu_count() or 0,
    }


def current_benchmarks(sizes: list[int], repeats: int) -> dict[str, Any]:
    import numpy as np

    import rhealpixdggs as rh
    from rhealpixdggs import numpy as rhnp

    results: dict[str, Any] = {
        "implementation": f"rhealpixdggs-rs {rh.__version__}",
        "parallelism_available": rhnp.PARALLELISM_AVAILABLE,
        "point_parallel_threshold": rhnp.POINT_PARALLEL_THRESHOLD,
        "points": {},
    }
    scalar_point = (-40.356, 175.611)
    results["scalar_point"] = scalar_measurement(
        lambda: rh.latlng_to_cell(*scalar_point, 12), repeats
    )
    for size in sizes:
        points = np.asarray(coordinates(size), dtype=np.float64)
        entries = {}
        for mode in (False, None, True):
            name = {False: "sequential", None: "automatic", True: "parallel"}[mode]
            entry = measurement(
                lambda mode=mode: rhnp.latlngs_to_cells(points, 9, parallel=mode),
                repeats,
            )
            entry["points_per_second"] = int(size * 1e9 / entry["median_ns"])
            entry["input_bytes"] = points.nbytes
            entry["output_bytes"] = size * 8
            entries[name] = entry
        results["points"][str(size)] = entries

    boundary_size = min(max(sizes), 4_096)
    boundary_points = np.asarray(coordinates(boundary_size), dtype=np.float64)
    boundary_cells = rhnp.latlngs_to_cells(boundary_points, 9, parallel=False)
    results["boundaries"] = {
        "count": boundary_size,
        "points_per_edge": 4,
        **measurement(
            lambda: rhnp.cells_to_boundaries(
                boundary_cells, points_per_edge=4, parallel=None
            ),
            repeats,
        ),
        "output_bytes": boundary_size * 12 * 2 * 8,
    }
    return results


def upstream_benchmarks(sizes: list[int], repeats: int, upstream_root: Path) -> dict[str, Any]:
    sys.path.insert(0, str(upstream_root.resolve()))
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from generate_upstream_corpus import _minimal_dependency_stubs

    _minimal_dependency_stubs()
    if "scipy" not in sys.modules:
        scipy = types.ModuleType("scipy")
        scipy.integrate = types.ModuleType("scipy.integrate")
        sys.modules["scipy"] = scipy
        sys.modules["scipy.integrate"] = scipy.integrate
    from rhealpixdggs.dggs import RHEALPixDGGS
    from rhealpixdggs.ellipsoids import WGS84_ELLIPSOID

    dggs = RHEALPixDGGS(ellipsoid=WGS84_ELLIPSOID)

    def point_to_cell(latitude: float, longitude: float, resolution: int) -> str:
        return str(dggs.cell_from_point(resolution, (longitude, latitude), plane=False))

    results: dict[str, Any] = {
        "implementation": "rhealpixdggs-py 0.6.0",
        "points": {},
    }
    results["scalar_point"] = scalar_measurement(
        lambda: point_to_cell(-40.356, 175.611, 12), repeats
    )
    for size in sizes:
        points = coordinates(size)
        entry = measurement(
            lambda: [point_to_cell(latitude, longitude, 9) for latitude, longitude in points],
            repeats,
        )
        entry["points_per_second"] = int(size * 1e9 / entry["median_ns"])
        results["points"][str(size)] = entry
    return results


def upstream_subprocess(
    script: Path, sizes: list[int], repeats: int, upstream_root: Path
) -> dict[str, Any]:
    completed = subprocess.run(
        [
            sys.executable,
            str(script),
            "--mode",
            "upstream",
            "--sizes",
            *(str(size) for size in sizes),
            "--repeats",
            str(repeats),
            "--upstream-root",
            str(upstream_root),
        ],
        check=True,
        capture_output=True,
        text=True,
        cwd=script.parent.parent,
    )
    return json.loads(completed.stdout)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("all", "current", "upstream"), default="all")
    parser.add_argument("--sizes", nargs="+", type=int, default=[256, 4_096, 65_536])
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--upstream-root", type=Path)
    arguments = parser.parse_args()
    if arguments.repeats < 1 or any(size < 1 for size in arguments.sizes):
        parser.error("repeats and sizes must be positive")

    if arguments.mode == "upstream":
        if arguments.upstream_root is None:
            parser.error("--upstream-root is required in upstream mode")
        report = upstream_benchmarks(
            arguments.sizes, arguments.repeats, arguments.upstream_root
        )
    elif arguments.mode == "current":
        report = current_benchmarks(arguments.sizes, arguments.repeats)
    else:
        if arguments.upstream_root is None:
            parser.error("--upstream-root is required in all mode")
        report = {
            "schema": "rhealpixdggs-rs-m2-benchmark-v1",
            "environment": environment(),
            "current": current_benchmarks(arguments.sizes, arguments.repeats),
            "upstream": upstream_subprocess(
                Path(__file__).resolve(),
                arguments.sizes,
                arguments.repeats,
                arguments.upstream_root,
            ),
        }
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
