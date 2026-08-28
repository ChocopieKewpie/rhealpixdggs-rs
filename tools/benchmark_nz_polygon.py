#!/usr/bin/env python3
"""Benchmark New Zealand polygon-to-GeoPackage conversion.

Run ``--mode all`` from the Rust environment and provide either a Python
executable with rHEALPixDGGS 0.6.0 installed or an unpacked 0.6.0 package root.
The two implementations run in separate processes because they intentionally
share the same ``rhealpixdggs`` import name.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time
import tracemalloc
from collections.abc import Callable, Iterable
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INPUT = ROOT / "benchmarks" / "data" / "new-zealand-simplified.geojson"


def progress(message: str) -> None:
    """Report long-running stages without contaminating JSON stdout."""
    print(f"[benchmark] {message}", file=sys.stderr, flush=True)


def measurement(call: Callable[[], Any], repeats: int) -> dict[str, int]:
    call()
    samples: list[int] = []
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
    return {
        "median_ns": int(statistics.median(samples)),
        "min_ns": min(samples),
        "max_ns": max(samples),
        "traced_peak_bytes": peak,
    }


def measured_stage(
    implementation: str,
    operation: str,
    call: Callable[[], Any],
    repeats: int,
) -> dict[str, int]:
    progress(f"{implementation}: {operation} started")
    result = measurement(call, repeats)
    seconds = result["median_ns"] / 1_000_000_000
    progress(f"{implementation}: {operation} finished ({seconds:.3f} s median)")
    return result


def environment() -> dict[str, str | int]:
    return {
        "os": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "python": platform.python_version(),
        "logical_cpus": os.cpu_count() or 0,
    }


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def cell_sha256(cells: Iterable[str]) -> str:
    return hashlib.sha256("\n".join(sorted(cells)).encode()).hexdigest()


def read_geometry(path: Path, layer: str | None = None) -> Any:
    import geopandas as gpd

    frame = gpd.read_file(path, layer=layer, engine="pyogrio")
    if frame.empty:
        raise ValueError("benchmark input contains no features")
    if frame.crs is None:
        raise ValueError("benchmark input has no CRS")
    frame = frame.to_crs("EPSG:4326")
    if hasattr(frame.geometry, "union_all"):
        return frame.geometry.union_all()
    return frame.geometry.unary_union


def polygon_parts(geometry: Any) -> list[Any]:
    if geometry.is_empty:
        return []
    if geometry.geom_type == "Polygon":
        return [geometry] if geometry.area > 0 else []
    if geometry.geom_type in {"MultiPolygon", "GeometryCollection"}:
        result: list[Any] = []
        for member in geometry.geoms:
            result.extend(polygon_parts(member))
        return result
    return []


def boundary_multipolygon(boundary: Iterable[Any]) -> Any:
    import shapely
    from shapely import affinity

    lonlats = [(float(point[0]), float(point[1])) for point in boundary]
    longitudes = [point[0] for point in lonlats]
    if max(longitudes) - min(longitudes) <= 180.0:
        parts = polygon_parts(shapely.make_valid(shapely.Polygon(lonlats)))
    else:
        shifted = [
            (longitude + 360.0 if longitude < 0.0 else longitude, latitude)
            for longitude, latitude in lonlats
        ]
        polygon = shapely.make_valid(shapely.Polygon(shifted))
        west = polygon.intersection(shapely.box(0.0, -90.0, 180.0, 90.0))
        east = polygon.intersection(shapely.box(180.0, -90.0, 360.0, 90.0))
        parts = polygon_parts(west)
        parts.extend(
            affinity.translate(part, xoff=-360.0) for part in polygon_parts(east)
        )
    if not parts:
        raise ValueError("cell boundary did not produce polygonal geometry")
    return shapely.MultiPolygon(parts)


def legacy_frame(cells: list[str], points_per_edge: int, dggs: Any) -> Any:
    import geopandas as gpd

    cell_objects = [
        dggs.cell((identifier[0], *(int(digit) for digit in identifier[1:])))
        for identifier in cells
    ]
    return gpd.GeoDataFrame(
        {
            "cell_id": cells,
            "resolution": [cell.resolution for cell in cell_objects],
            "area_m2": [cell.area(plane=False) for cell in cell_objects],
        },
        geometry=[
            boundary_multipolygon(
                cell.boundary(n=points_per_edge, plane=False, interior=False)
            )
            for cell in cell_objects
        ],
        crs="EPSG:4326",
    )


def write_frame(frame: Any, output: Path) -> Path:
    output.parent.mkdir(parents=True, exist_ok=True)
    frame.to_file(
        output,
        layer="rhealpix_cells",
        driver="GPKG",
        engine="pyogrio",
    )
    return output


def unique_writer(directory: Path, prefix: str) -> Callable[[Any], Path]:
    counter = 0

    def write(frame: Any) -> Path:
        nonlocal counter
        counter += 1
        return write_frame(frame, directory / f"{prefix}-{counter}.gpkg")

    return write


def result_common(
    implementation: str,
    boundary_contract: str,
    input_path: Path,
    input_layer: str | None,
    output: Path,
    resolution: int,
    points_per_edge: int,
    cells: list[str],
    timings: dict[str, dict[str, int]],
) -> dict[str, Any]:
    return {
        "implementation": implementation,
        "boundary_contract": boundary_contract,
        "environment": environment(),
        "input": str(input_path.resolve()),
        "input_layer": input_layer,
        "input_sha256": file_sha256(input_path),
        "resolution": resolution,
        "points_per_edge": points_per_edge,
        "cell_count": len(cells),
        "cell_ids_sha256": cell_sha256(cells),
        "output": str(output.resolve()),
        "output_bytes": output.stat().st_size,
        "output_sha256": file_sha256(output),
        "timings": timings,
        "_cell_ids": cells,
    }


def benchmark_current(arguments: argparse.Namespace) -> dict[str, Any]:
    import rhealpixdggs as rh
    from rhealpixdggs import geo

    geometry = read_geometry(arguments.input, arguments.input_layer)

    def cover() -> list[str]:
        return geo.geometry_to_cells(geometry, arguments.resolution)

    progress("Rust 0.8.0: preparing initial cell coverage")
    cells = cover()

    def build() -> Any:
        return geo.cells_to_geodataframe(
            cells,
            points_per_edge=arguments.points_per_edge,
            parallel=None,
        )

    progress(f"Rust 0.8.0: preparing boundaries for {len(cells):,} cells")
    frame = build()

    with tempfile.TemporaryDirectory(prefix="rhealpix-rust-benchmark-") as raw_temp:
        temporary = Path(raw_temp)
        write = unique_writer(temporary, "write")
        end_to_end_write = unique_writer(temporary, "end-to-end")

        def end_to_end() -> Any:
            covered = geo.geometry_to_cells(
                read_geometry(arguments.input, arguments.input_layer),
                arguments.resolution,
            )
            converted = geo.cells_to_geodataframe(
                covered,
                points_per_edge=arguments.points_per_edge,
                parallel=None,
            )
            return end_to_end_write(converted)

        timings = {
            "read": measured_stage(
                "Rust 0.8.0",
                "input read",
                lambda: read_geometry(arguments.input, arguments.input_layer),
                arguments.repeats,
            ),
            "cover": measured_stage(
                "Rust 0.8.0", "polygon coverage", cover, arguments.repeats
            ),
            "boundaries": measured_stage(
                "Rust 0.8.0", "cell boundaries", build, arguments.repeats
            ),
            "write_gpkg": measured_stage(
                "Rust 0.8.0",
                "GeoPackage write",
                lambda: write(frame),
                arguments.repeats,
            ),
            "end_to_end": measured_stage(
                "Rust 0.8.0", "end-to-end pipeline", end_to_end, arguments.repeats
            ),
        }

    output = arguments.output_dir / "new-zealand-rhealpix-rust.gpkg"
    geo.write_geopackage(frame, output, overwrite=arguments.overwrite)
    return result_common(
        f"rhealpixdggs-rs {rh.__version__}",
        "exact 4 * points_per_edge - 4 vertices for every cell",
        arguments.input,
        arguments.input_layer,
        output,
        arguments.resolution,
        arguments.points_per_edge,
        cells,
        timings,
    )


def import_legacy(upstream_root: Path | None) -> tuple[Any, Callable[..., Any], str]:
    if upstream_root is not None:
        sys.path.insert(0, str(upstream_root.resolve()))
    try:
        from rhealpixdggs.dggs import WGS84_003
        from rhealpixdggs.rhp_wrappers import polyfill
    except (ImportError, ModuleNotFoundError) as error:
        raise RuntimeError(
            "legacy mode requires rHEALPixDGGS 0.6.0 in this Python environment "
            "or --upstream-root pointing to an unpacked 0.6.0 package"
        ) from error
    try:
        version = importlib.metadata.version("rHEALPixDGGS")
    except importlib.metadata.PackageNotFoundError:
        version = "0.6.0 source tree"
    return WGS84_003, polyfill, version


def benchmark_legacy(arguments: argparse.Namespace) -> dict[str, Any]:
    dggs, polyfill, version = import_legacy(arguments.upstream_root)
    geometry = read_geometry(arguments.input, arguments.input_layer)

    def cover() -> list[str]:
        result = polyfill(
            geometry,
            res=arguments.resolution,
            plane=False,
            compress=False,
            verbose=False,
            dggs=dggs,
        )
        if result is None:
            raise RuntimeError("legacy polyfill rejected the benchmark geometry")
        return sorted(result)

    progress("legacy 0.6.0: preparing initial cell coverage")
    cells = cover()

    def build() -> Any:
        return legacy_frame(cells, arguments.points_per_edge, dggs)

    progress(f"legacy 0.6.0: preparing boundaries for {len(cells):,} cells")
    frame = build()

    with tempfile.TemporaryDirectory(prefix="rhealpix-legacy-benchmark-") as raw_temp:
        temporary = Path(raw_temp)
        write = unique_writer(temporary, "write")
        end_to_end_write = unique_writer(temporary, "end-to-end")

        def end_to_end() -> Any:
            loaded = read_geometry(arguments.input, arguments.input_layer)
            result = polyfill(
                loaded,
                res=arguments.resolution,
                plane=False,
                compress=False,
                verbose=False,
                dggs=dggs,
            )
            if result is None:
                raise RuntimeError("legacy polyfill rejected the benchmark geometry")
            return end_to_end_write(
                legacy_frame(sorted(result), arguments.points_per_edge, dggs)
            )

        timings = {
            "read": measured_stage(
                "legacy 0.6.0",
                "input read",
                lambda: read_geometry(arguments.input, arguments.input_layer),
                arguments.repeats,
            ),
            "cover": measured_stage(
                "legacy 0.6.0", "polygon coverage", cover, arguments.repeats
            ),
            "boundaries": measured_stage(
                "legacy 0.6.0", "cell boundaries", build, arguments.repeats
            ),
            "write_gpkg": measured_stage(
                "legacy 0.6.0",
                "GeoPackage write",
                lambda: write(frame),
                arguments.repeats,
            ),
            "end_to_end": measured_stage(
                "legacy 0.6.0", "end-to-end pipeline", end_to_end, arguments.repeats
            ),
        }

    output = arguments.output_dir / "new-zealand-rhealpix-legacy.gpkg"
    if output.exists() and not arguments.overwrite:
        raise FileExistsError(
            f"output already exists: {output}; pass --overwrite to replace it"
        )
    write_frame(frame, output)
    return result_common(
        f"rHEALPixDGGS {version}",
        "0.6.0 Cell.boundary; quad/cap cells use four-vertex shortcut",
        arguments.input,
        arguments.input_layer,
        output,
        arguments.resolution,
        arguments.points_per_edge,
        cells,
        timings,
    )


def legacy_subprocess(arguments: argparse.Namespace) -> dict[str, Any]:
    executable = arguments.legacy_python or Path(sys.executable)
    command = [
        str(executable),
        str(Path(__file__).resolve()),
        "--mode",
        "legacy",
        "--input",
        str(arguments.input.resolve()),
        "--output-dir",
        str(arguments.output_dir.resolve()),
        "--resolution",
        str(arguments.resolution),
        "--points-per-edge",
        str(arguments.points_per_edge),
        "--repeats",
        str(arguments.repeats),
        "--include-cells",
    ]
    if arguments.input_layer is not None:
        command.extend(["--input-layer", arguments.input_layer])
    if arguments.upstream_root is not None:
        command.extend(["--upstream-root", str(arguments.upstream_root.resolve())])
    if arguments.overwrite:
        command.append("--overwrite")
    completed = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            "legacy benchmark subprocess failed:\n"
            + (completed.stderr.strip() or completed.stdout.strip())
        )
    return json.loads(completed.stdout)


def speedup(legacy: dict[str, Any], current: dict[str, Any], operation: str) -> float:
    return round(
        legacy["timings"][operation]["median_ns"]
        / current["timings"][operation]["median_ns"],
        3,
    )


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser()
    command.add_argument("--mode", choices=("all", "current", "legacy"), default="all")
    command.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    command.add_argument("--input-layer")
    command.add_argument("--output-dir", type=Path, default=ROOT / "benchmark-results")
    command.add_argument("--resolution", type=int, default=6)
    command.add_argument("--points-per-edge", type=int, default=4)
    command.add_argument("--repeats", type=int, default=3)
    command.add_argument("--legacy-python", type=Path)
    command.add_argument("--upstream-root", type=Path)
    command.add_argument("--overwrite", action="store_true")
    command.add_argument("--include-cells", action="store_true", help=argparse.SUPPRESS)
    return command


def main() -> None:
    arguments = parser().parse_args()
    if arguments.repeats < 1:
        raise SystemExit("--repeats must be positive")
    if arguments.points_per_edge < 2:
        raise SystemExit("--points-per-edge must be at least 2")
    if not arguments.input.is_file():
        raise SystemExit(f"input does not exist: {arguments.input}")
    arguments.output_dir.mkdir(parents=True, exist_ok=True)

    if arguments.mode == "current":
        report: dict[str, Any] = benchmark_current(arguments)
    elif arguments.mode == "legacy":
        report = benchmark_legacy(arguments)
    else:
        if arguments.legacy_python is None and arguments.upstream_root is None:
            raise SystemExit(
                "--mode all requires --legacy-python or --upstream-root so the "
                "0.6.0 package can be imported separately"
            )
        current = benchmark_current(arguments)
        legacy = legacy_subprocess(arguments)
        current_cells = set(current.pop("_cell_ids"))
        legacy_cells = set(legacy.pop("_cell_ids"))
        report = {
            "schema": "rhealpixdggs-rs-nz-polygon-benchmark-v1",
            "current": current,
            "legacy": legacy,
            "comparison": {
                "same_cells": current_cells == legacy_cells,
                "current_only_count": len(current_cells - legacy_cells),
                "legacy_only_count": len(legacy_cells - current_cells),
                "current_only_sample": sorted(current_cells - legacy_cells)[:20],
                "legacy_only_sample": sorted(legacy_cells - current_cells)[:20],
                "cover_speedup": speedup(legacy, current, "cover"),
                "boundary_speedup": speedup(legacy, current, "boundaries"),
                "end_to_end_speedup": speedup(legacy, current, "end_to_end"),
            },
        }

    if not arguments.include_cells:
        report.pop("_cell_ids", None)
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
