#!/usr/bin/env python3
"""Generate line, region, and polygon fixtures from rHEALPixDGGS 0.6.0."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import sys
import types
from pathlib import Path
from typing import Any

UPSTREAM_NAME = "rHEALPixDGGS"
UPSTREAM_VERSION = "0.6.0"
DEFAULT_OUTPUT = (
    Path(__file__).resolve().parents[1]
    / "tests"
    / "fixtures"
    / "rhealpixdggs-py-0.6.0"
    / "coverage-v1.json"
)

REGIONS = [
    {
        "id": "northern_quad",
        "configuration": [0, 0],
        "resolution": 1,
        "upper_left": [0.0, 60.0],
        "lower_right": [90.0, 0.0],
        "plane": False,
    },
    {
        "id": "southern_lune",
        "configuration": [0, 0],
        "resolution": 1,
        "upper_left": [0.0, -30.0],
        "lower_right": [90.0, -90.0],
        "plane": False,
    },
    {
        "id": "southern_cap",
        "configuration": [0, 0],
        "resolution": 1,
        "upper_left": [-180.0, -36.0],
        "lower_right": [-180.0, -90.0],
        "plane": False,
    },
    {
        "id": "custom_polar_layout",
        "configuration": [1, 3],
        "resolution": 1,
        "upper_left": [0.0, 60.0],
        "lower_right": [90.0, 0.0],
        "plane": False,
    },
]

LINES = [
    {
        "id": "upstream_polar_doctest",
        "configuration": [0, 0],
        "resolution": 3,
        "start": [-89.669615, 86.549596],
        "end": [-134.0, 86.0],
        "plane": False,
    },
    {
        "id": "upstream_wrapper_doctest",
        "configuration": [0, 0],
        "resolution": 9,
        "start": [-176.260506, -43.738058],
        "end": [-176.258807, -43.738379],
        "plane": False,
    },
    {
        "id": "equatorial_diagonal",
        "configuration": [0, 0],
        "resolution": 3,
        "start": [0.0, 0.0],
        "end": [40.0, 20.0],
        "plane": False,
    },
    {
        "id": "cell_edge_start",
        "configuration": [0, 0],
        "resolution": 3,
        "start": [10.0, -20.0],
        "end": [80.0, 40.0],
        "plane": False,
    },
    {
        "id": "custom_polar_layout",
        "configuration": [1, 3],
        "resolution": 3,
        "start": [-89.669615, 86.549596],
        "end": [-134.0, 86.0],
        "plane": False,
    },
]

POLYGONS = [
    {
        "id": "unit_square_r4",
        "resolution": 4,
        "exterior": [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        "holes": [],
    },
    {
        "id": "unit_square_r5",
        "resolution": 5,
        "exterior": [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        "holes": [],
    },
    {
        "id": "unit_square_r6",
        "resolution": 6,
        "exterior": [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        "holes": [],
    },
    {
        "id": "unit_square_with_hole",
        "resolution": 6,
        "exterior": [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        "holes": [
            [[0.3, 0.3], [0.3, 0.7], [0.7, 0.7], [0.7, 0.3]],
        ],
    },
]

POINT_EDGES = [
    {"longitude": 10.0, "latitude": -20.0, "resolution": 3},
    {"longitude": 10.0, "latitude": 20.0, "resolution": 3},
    {"longitude": 20.0, "latitude": 10.0, "resolution": 3},
]


def _cross(left: tuple[float, float], right: tuple[float, float]) -> float:
    return left[0] * right[1] - left[1] * right[0]


def _subtract(
    left: tuple[float, float], right: tuple[float, float]
) -> tuple[float, float]:
    return left[0] - right[0], left[1] - right[1]


def _segments_intersect(
    a: tuple[float, float],
    b: tuple[float, float],
    c: tuple[float, float],
    d: tuple[float, float],
) -> bool:
    def orientation(p: Any, q: Any, r: Any) -> float:
        return _cross(_subtract(q, p), _subtract(r, p))

    def on_segment(p: Any, q: Any, r: Any) -> bool:
        return (
            min(p[0], r[0]) - 1e-10 <= q[0] <= max(p[0], r[0]) + 1e-10
            and min(p[1], r[1]) - 1e-10 <= q[1] <= max(p[1], r[1]) + 1e-10
        )

    values = (
        orientation(a, b, c),
        orientation(a, b, d),
        orientation(c, d, a),
        orientation(c, d, b),
    )
    if (values[0] > 0) != (values[1] > 0) and (values[2] > 0) != (
        values[3] > 0
    ):
        return True
    return any(
        abs(value) <= 1e-10 and on_segment(*points)
        for value, points in zip(
            values,
            ((a, c, b), (a, d, b), (c, a, d), (c, b, d)),
        )
    )


class _LineString:
    def __init__(self, points: Any) -> None:
        self.points = list(points)

    def intersects(self, other: _LineString) -> bool:
        return any(
            _segments_intersect(a, b, c, d)
            for a, b in zip(self.points, self.points[1:])
            for c, d in zip(other.points, other.points[1:])
        )


def _dependency_stubs() -> None:
    shapely = types.ModuleType("shapely")
    shapely.LineString = _LineString
    shapely.Polygon = type("Polygon", (), {})
    sys.modules["shapely"] = shapely

    pyproj = types.ModuleType("pyproj")
    pyproj.get_ellps_map = lambda: {
        "WGS84": {"a": 6_378_137.0, "rf": 298.257_223_563},
        "sphere": {"a": 6_371_000.0},
    }
    pyproj.Proj = type("UnavailableProj", (), {})
    sys.modules["pyproj"] = pyproj


def _point_in_ring(point: tuple[float, float], ring: list[list[float]]) -> bool:
    inside = False
    for start, end in zip(ring, ring[1:] + ring[:1]):
        if (start[1] > point[1]) != (end[1] > point[1]):
            crossing = (end[0] - start[0]) * (point[1] - start[1]) / (
                end[1] - start[1]
            ) + start[0]
            if point[0] < crossing:
                inside = not inside
    return inside


def build_corpus(upstream_root: Path) -> dict[str, Any]:
    sys.path.insert(0, str(upstream_root.resolve()))
    _dependency_stubs()
    version = importlib.metadata.version(UPSTREAM_NAME)
    if version != UPSTREAM_VERSION:
        raise RuntimeError(f"expected {UPSTREAM_NAME} {UPSTREAM_VERSION}, got {version}")

    from rhealpixdggs.dggs import RHEALPixDGGS

    regions = []
    for case in REGIONS:
        dggs = RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        regions.append(
            {
                **case,
                "cells": [
                    [str(cell) for cell in row]
                    for row in dggs.cells_from_region(
                        case["resolution"],
                        tuple(case["upper_left"]),
                        tuple(case["lower_right"]),
                        case["plane"],
                    )
                ],
            }
        )

    lines = []
    for case in LINES:
        dggs = RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        lines.append(
            {
                **case,
                "cells": [
                    str(cell)
                    for cell in dggs.cells_from_line(
                        case["resolution"],
                        case["start"],
                        case["end"],
                        case["plane"],
                    )
                ],
            }
        )

    canonical = RHEALPixDGGS()
    point_edges = [
        {
            **case,
            "cell": str(
                canonical.cell_from_point(
                    case["resolution"],
                    (case["longitude"], case["latitude"]),
                    False,
                )
            ),
        }
        for case in POINT_EDGES
    ]
    polygons = []
    for case in POLYGONS:
        rings = [case["exterior"], *case["holes"]]
        west = min(point[0] for ring in rings for point in ring)
        east = max(point[0] for ring in rings for point in ring)
        south = min(point[1] for ring in rings for point in ring)
        north = max(point[1] for ring in rings for point in ring)
        candidates = (
            cell
            for row in canonical.cells_from_region(
                case["resolution"], (west, north), (east, south), False
            )
            for cell in row
        )
        cells = []
        for cell in candidates:
            centroid = tuple(float(value) for value in cell.centroid(plane=False))
            if _point_in_ring(centroid, case["exterior"]) and not any(
                _point_in_ring(centroid, hole) for hole in case["holes"]
            ):
                cells.append(str(cell))
        polygons.append({**case, "cells": sorted(set(cells))})

    return {
        "$schema": "coverage-schema-v1.json",
        "schema_version": 1,
        "corpus_version": "1.0.0",
        "upstream": {
            "distribution": UPSTREAM_NAME,
            "version": version,
            "source_url": "https://github.com/manaakiwhenua/rhealpixdggs-py",
        },
        "contract": {
            "polygon_selection": "strict ellipsoidal centroid containment",
            "line_interpretation": "straight segments in coordinate space",
            "known_upstream_limits_corrected_outside_corpus": [
                "antimeridian-crossing lines and polygons",
                "lines crossing polar cap cells",
            ],
        },
        "counts": {
            "point_edges": len(point_edges),
            "regions": len(regions),
            "lines": len(lines),
            "polygons": len(polygons),
        },
        "point_edges": point_edges,
        "regions": regions,
        "lines": lines,
        "polygons": polygons,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    rendered = json.dumps(build_corpus(arguments.upstream_root), indent=2) + "\n"
    checksum_path = arguments.output.with_suffix(".sha256")
    checksum_record = (
        f"{hashlib.sha256(rendered.encode()).hexdigest()}  {arguments.output.name}\n"
    )
    if arguments.check:
        current = (
            arguments.output.exists()
            and arguments.output.read_text() == rendered
            and checksum_path.exists()
            and checksum_path.read_text() == checksum_record
        )
        if not current:
            print(f"coverage corpus is stale: {arguments.output}", file=sys.stderr)
            return 1
        print(f"coverage corpus is current: {arguments.output}")
        return 0
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(rendered)
    checksum_path.write_text(checksum_record)
    print(arguments.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
