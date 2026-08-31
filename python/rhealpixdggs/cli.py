"""Command-line vector conversion for the optional geospatial adapter."""

from __future__ import annotations

import argparse
from pathlib import Path

from .geo import polygon_file_to_geopackage


def parser() -> argparse.ArgumentParser:
    command = argparse.ArgumentParser(
        prog="rhealpix-to-gpkg",
        description="Cover polygon features with rHEALPix cells and write a GeoPackage.",
    )
    command.add_argument("input", type=Path, help="input polygon vector file")
    command.add_argument("output", type=Path, help="output .gpkg file")
    command.add_argument("--resolution", "-r", type=int, required=True)
    command.add_argument("--input-layer")
    command.add_argument("--output-layer", default="rhealpix_cells")
    command.add_argument("--points-per-edge", type=int, default=4)
    command.add_argument("--compact", action="store_true")
    command.add_argument(
        "--coverage-mode",
        choices=("centroid", "intersects"),
        default="centroid",
        help="cell selection rule (default: centroid)",
    )
    command.add_argument(
        "--parallel",
        choices=("auto", "yes", "no"),
        default="auto",
        help="boundary conversion parallelism (default: auto)",
    )
    command.add_argument("--overwrite", action="store_true")
    return command


def main() -> None:
    arguments = parser().parse_args()
    parallel = {"auto": None, "yes": True, "no": False}[arguments.parallel]
    frame = polygon_file_to_geopackage(
        arguments.input,
        arguments.output,
        arguments.resolution,
        input_layer=arguments.input_layer,
        output_layer=arguments.output_layer,
        compact=arguments.compact,
        points_per_edge=arguments.points_per_edge,
        parallel=parallel,
        overwrite=arguments.overwrite,
        coverage_mode=arguments.coverage_mode,
    )
    print(f"wrote {len(frame):,} cells to {arguments.output}")


if __name__ == "__main__":
    main()
