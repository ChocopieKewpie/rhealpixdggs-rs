#!/usr/bin/env python3
"""Generate deterministic object-facade fixtures from rHEALPixDGGS 0.6.0."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import sys
from pathlib import Path
from typing import Any

from generate_upstream_corpus import _minimal_dependency_stubs

UPSTREAM_NAME = "rHEALPixDGGS"
UPSTREAM_VERSION = "0.6.0"
DEFAULT_OUTPUT = (
    Path(__file__).resolve().parents[1]
    / "tests"
    / "fixtures"
    / "rhealpixdggs-py-0.6.0"
    / "facade-v1.json"
)


def _point(value: Any) -> list[float]:
    return [float(value[0]), float(value[1])]


def _points(values: Any) -> list[list[float]]:
    return [_point(value) for value in values]


def _triple(value: Any) -> list[float]:
    return [float(value[0]), float(value[1]), float(value[2])]


def _matrix(values: Any) -> list[list[list[float]]]:
    return [_points(row) for row in values]


def _suid(identifier: str) -> tuple[str | int, ...]:
    return (identifier[0], *(int(digit) for digit in identifier[1:]))


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_corpus(upstream_root: Path | None) -> dict[str, Any]:
    if upstream_root is not None:
        sys.path.insert(0, str(upstream_root.resolve()))
    _minimal_dependency_stubs()
    installed_version = importlib.metadata.version(UPSTREAM_NAME)
    if installed_version != UPSTREAM_VERSION:
        raise RuntimeError(
            f"expected {UPSTREAM_NAME} {UPSTREAM_VERSION}, got {installed_version}"
        )

    from rhealpixdggs.dggs import RHEALPixDGGS

    package_root = Path(sys.modules["rhealpixdggs"].__file__).resolve().parent
    sources = {
        name: _sha256(package_root / name)
        for name in [
            "cell.py",
            "dggs.py",
            "ellipsoids.py",
            "pj_healpix.py",
            "pj_rhealpix.py",
            "projection_wrapper.py",
        ]
    }
    configurations = [[0, 0], [1, 3]]
    projections: list[dict[str, Any]] = []
    triangle_transforms: list[dict[str, Any]] = []
    triangle_cases: list[dict[str, Any]] = []
    cartesian_cases: list[dict[str, Any]] = []
    region_parents: list[dict[str, Any]] = []
    latitude_cases: list[dict[str, Any]] = []
    meridians: list[dict[str, Any]] = []
    parallels: list[dict[str, Any]] = []
    cell_cases: list[dict[str, Any]] = []

    for north_square, south_square in configurations:
        dggs = RHEALPixDGGS(
            north_square=north_square,
            south_square=south_square,
        )
        for kind in ["healpix", "rhealpix"]:
            if kind == "healpix" and north_square != 0:
                continue
            function = getattr(dggs, kind)
            for point in [(0.0, 0.0), (175.611, -40.356), (45.0, 89.0)]:
                projected = function(*point)
                projections.append(
                    {
                        "configuration": [north_square, south_square],
                        "projection": kind,
                        "lonlat": list(point),
                        "projected": _point(projected),
                        "roundtrip": _point(function(*projected, inverse=True)),
                    }
                )

        for point in [(0.0, 0.0), dggs.healpix(45.0, 89.0)]:
            transformed = dggs.combine_triangles(*point)
            triangle_transforms.append(
                {
                    "configuration": [north_square, south_square],
                    "healpix": _point(point),
                    "rhealpix": _point(transformed),
                    "roundtrip": _point(
                        dggs.combine_triangles(*transformed, inverse=True)
                    ),
                }
            )
        for identifier in ["N7", "N3", "P3", "S52"]:
            nucleus = dggs.cell(_suid(identifier)).nucleus()
            number, region = dggs.triangle(*nucleus, inverse=True)
            triangle_cases.append(
                {
                    "configuration": [north_square, south_square],
                    "point": _point(nucleus),
                    "inverse": True,
                    "number": number,
                    "region": region,
                }
            )
        for lonlat in [(0.0, 0.0), (175.611, -40.356), (45.0, 89.0)]:
            projected = dggs.rhealpix(*lonlat)
            cartesian_cases.append(
                {
                    "configuration": [north_square, south_square],
                    "lonlat": list(lonlat),
                    "projected": _point(projected),
                    "xyz_lonlat": _triple(dggs.xyz(*lonlat, lonlat=True)),
                    "xyz_projected": _triple(dggs.xyz(*projected)),
                    "cube_lonlat": _triple(dggs.xyz_cube(*lonlat, lonlat=True)),
                    "cube_projected": _triple(dggs.xyz_cube(*projected)),
                }
            )

        inset_cell = dggs.cell(_suid("Q3"))
        upper_left = inset_cell.ul_vertex()
        width = inset_cell.width()
        inset = width * 1e-8
        region_inputs = [
            (
                "projected_q3",
                upper_left,
                (
                    upper_left[0] + width - inset,
                    upper_left[1] - width + inset,
                ),
                True,
            ),
            ("geographic_quad", (-10.0, 10.0), (10.0, -10.0), False),
            ("north_cap", (-180.0, 90.0), (-180.0, 60.0), False),
        ]
        for name, upper, lower, plane in region_inputs:
            parent = dggs.cell_from_region(upper, lower, plane=plane)
            region_parents.append(
                {
                    "id": name,
                    "configuration": [north_square, south_square],
                    "upper_left": list(upper),
                    "lower_right": list(lower),
                    "plane": plane,
                    "cell": None if parent is None else str(parent),
                }
            )

        for nucleus in [True, False]:
            latitude_cases.append(
                {
                    "configuration": [north_square, south_square],
                    "resolution": 1,
                    "minimum": -90.0,
                    "maximum": 90.0,
                    "nucleus": nucleus,
                    "plane": False,
                    "values": [
                        float(value)
                        for value in dggs.cell_latitudes(
                            1, -90.0, 90.0, nucleus=nucleus, plane=False
                        )
                    ],
                }
            )

        meridians.append(
            {
                "configuration": [north_square, south_square],
                "resolution": 1,
                "longitude": 5.729577951308233,
                "latitude_min": -90.0,
                "latitude_max": 90.0,
                "cells": [
                    str(cell)
                    for cell in dggs.cells_from_meridian(
                        1, 5.729577951308233, -90.0, 90.0
                    )
                ],
            }
        )
        parallels.append(
            {
                "configuration": [north_square, south_square],
                "resolution": 1,
                "latitude": 60.0,
                "longitude_min": -180.0,
                "longitude_max": 180.0,
                "cells": [
                    str(cell)
                    for cell in dggs.cells_from_parallel(1, 60.0, -180.0, 180.0)
                ],
            }
        )

        for identifier in ["N", "N43", "S62", "Q57", "N73"]:
            cell = dggs.cell(_suid(identifier))
            row_suid, column_suid = cell.suid_rowcol()
            cell_cases.append(
                {
                    "configuration": [north_square, south_square],
                    "cell": identifier,
                    "row_suid": list(row_suid),
                    "column_suid": list(column_suid),
                    "rotations": [str(cell.rotate(turns)) for turns in range(4)],
                    "upper_left_projected": _point(cell.ul_vertex()),
                    "upper_left_lonlat": _point(cell.ul_vertex(plane=False)),
                    "northwest_projected": _point(cell.nw_vertex()),
                    "northwest_lonlat": _point(cell.nw_vertex(plane=False)),
                    "xy_range": [list(pair) for pair in cell.xy_range()],
                    "interior_projected_n3": _matrix(cell.interior(n=3)),
                    "interior_lonlat_flat_n3": _points(
                        cell.interior(n=3, plane=False, flatten=True)
                    ),
                    "contains_projected_nucleus": bool(cell.contains(cell.nucleus())),
                    "contains_lonlat_nucleus": bool(
                        cell.contains(cell.nucleus(plane=False), plane=False)
                    ),
                    "meridians": {
                        str(value): bool(cell.intersects_meridian(value))
                        for value in [-180.0, -90.0, 0.0, 90.0, 180.0]
                    },
                    "parallels": {
                        str(value): bool(cell.intersects_parallel(value))
                        for value in [-90.0, -60.0, 0.0, 60.0, 90.0]
                    },
                }
            )

    canonical = RHEALPixDGGS()
    points = [
        canonical.cell(_suid(identifier)).nucleus()
        for identifier in ["N021", "P733", "N021"]
    ]
    minimal_covers = [
        {
            "resolution": resolution,
            "points": _points(points),
            "plane": True,
            "cells": [
                str(cell) for cell in canonical.minimal_cover(resolution, points)
            ],
        }
        for resolution in [0, 1, 2, 3, 4]
    ]
    intervals = [
        {
            "start": "N1",
            "end": "N",
            "cells": [
                str(cell)
                for cell in canonical.interval(
                    canonical.cell(_suid("N1")), canonical.cell(_suid("N"))
                )
            ],
        },
        {
            "start": "N08",
            "end": "N1",
            "cells": [
                str(cell)
                for cell in canonical.interval(
                    canonical.cell(_suid("N08")), canonical.cell(_suid("N1"))
                )
            ],
        },
    ]
    overlap_pairs = [
        {
            "left": left,
            "right": right,
            "overlaps": bool(
                canonical.cell(_suid(left)).overlaps(canonical.cell(_suid(right)))
            ),
        }
        for left, right in [
            ("N", "N43"),
            ("N43", "N"),
            ("N43", "N430"),
            ("N43", "N44"),
            ("N43", "S43"),
        ]
    ]

    return {
        "schema_version": 1,
        "upstream": {
            "distribution": UPSTREAM_NAME,
            "version": UPSTREAM_VERSION,
            "source_sha256": sources,
        },
        "error_budget": {
            "projected_absolute_metres": 2e-8,
            "geographic_absolute_degrees": 2e-10,
        },
        "configurations": configurations,
        "projections": projections,
        "triangle_transforms": triangle_transforms,
        "triangles": triangle_cases,
        "cartesian": cartesian_cases,
        "region_parents": region_parents,
        "cell_latitudes": latitude_cases,
        "meridians": meridians,
        "parallels": parallels,
        "minimal_covers": minimal_covers,
        "intervals": intervals,
        "cells": cell_cases,
        "overlaps": overlap_pairs,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--upstream-root", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    payload = (
        json.dumps(build_corpus(args.upstream_root), indent=2, sort_keys=True)
        + "\n"
    ).encode()
    checksum = hashlib.sha256(payload).hexdigest()
    checksum_path = args.output.with_suffix(".sha256")
    checksum_payload = f"{checksum}  {args.output.name}\n".encode()
    if args.check:
        if not args.output.exists() or args.output.read_bytes() != payload:
            raise SystemExit(f"fixture is stale: {args.output}")
        if not checksum_path.exists() or checksum_path.read_bytes() != checksum_payload:
            raise SystemExit(f"checksum is stale: {checksum_path}")
        return
    args.output.write_bytes(payload)
    checksum_path.write_bytes(checksum_payload)


if __name__ == "__main__":
    main()
