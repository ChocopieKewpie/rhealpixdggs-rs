#!/usr/bin/env python3
"""Generate the language-neutral rHEALPix upstream conformance corpus."""

from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import importlib.util
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
    / "conformance-v1.json"
)

POINTS = [
    (-180.0, 0.0),
    (-179.999, 72.5),
    (-122.4194, 37.7749),
    (-90.0, -41.0),
    (-45.0, 45.0),
    (0.0, 0.0),
    (45.0, 89.0),
    (90.0, -45.0),
    (120.0, -89.0),
    (174.7633, -36.8485),
    (175.611, -40.356),
    (179.999, -72.5),
]
RESOLUTIONS = [0, 1, 2, 3, 8, 12, 15]
GEOMETRY_CELLS = [
    "N",
    "S",
    "P57",
    "Q77",
    "N0",
    "S0",
    "N4",
    "S4",
    "N43",
    "S43",
    "N62",
    "S62",
    "N6",
]
TOPOLOGY_CELLS = [
    "N",
    "O",
    "S",
    "N0",
    "N08",
    "N82",
    "N88",
    "O0",
    "P2",
    "Q381",
    "R888",
    "S888",
    "N622446670001",
    "S407138265401",
    "S888888888888888",
]
TRAVERSAL_RESOLUTIONS = [0, 1, 3, 15]
MIXED_ORDER_CELLS = ["N", "N0", "N00", "N01", "N08", "N1", "O0"]


def _minimal_dependency_stubs() -> None:
    """Install import-only stubs for unused optional dependency surfaces."""
    if importlib.util.find_spec("shapely") is None:
        shapely = types.ModuleType("shapely")
        shapely.LineString = type("LineString", (), {})
        shapely.Polygon = type("Polygon", (), {})
        sys.modules["shapely"] = shapely
    if importlib.util.find_spec("pyproj") is None:
        pyproj = types.ModuleType("pyproj")
        pyproj.get_ellps_map = lambda: {
            "WGS84": {"a": 6_378_137.0, "rf": 298.257_223_563},
            "sphere": {"a": 6_371_000.0},
        }

        class UnavailableProj:
            def __init__(self, *_: object, **__: object) -> None:
                raise RuntimeError("the corpus must use upstream's homemade projection")

        pyproj.Proj = UnavailableProj
        sys.modules["pyproj"] = pyproj


def _suid(identifier: str) -> tuple[str | int, ...]:
    return (identifier[0], *(int(digit) for digit in identifier[1:]))


def _point(value: Any) -> list[float]:
    return [float(value[0]), float(value[1])]


def _points(values: Any) -> list[list[float]]:
    return [_point(value) for value in values]


def _cell_name(value: Any) -> str | None:
    return None if value is None else str(value)


def _cell_call(cell: Any, method: str, *args: object) -> tuple[str | None, str | None]:
    try:
        return _cell_name(getattr(cell, method)(*args)), None
    except AttributeError as error:
        return None, f"{type(error).__name__}: {error}"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_corpus(upstream_root: Path | None, use_stubs: bool) -> dict[str, Any]:
    if upstream_root is not None:
        sys.path.insert(0, str(upstream_root.resolve()))
    if use_stubs:
        _minimal_dependency_stubs()

    installed_version = importlib.metadata.version(UPSTREAM_NAME)
    if installed_version != UPSTREAM_VERSION:
        raise RuntimeError(
            f"expected {UPSTREAM_NAME} {UPSTREAM_VERSION}, got {installed_version}"
        )

    from rhealpixdggs.dggs import RHEALPixDGGS

    package_root = Path(sys.modules["rhealpixdggs"].__file__).resolve().parent
    source_files = [
        "cell.py",
        "dggs.py",
        "ellipsoids.py",
        "pj_healpix.py",
        "pj_rhealpix.py",
        "projection_wrapper.py",
        "utils.py",
    ]
    configurations = [
        {
            "id": f"ns{north_square}_ss{south_square}",
            "north_square": north_square,
            "south_square": south_square,
        }
        for north_square in range(4)
        for south_square in range(4)
    ]

    point_indexing: list[dict[str, Any]] = []
    cell_geometry: list[dict[str, Any]] = []
    for configuration in configurations:
        dggs = RHEALPixDGGS(
            north_square=configuration["north_square"],
            south_square=configuration["south_square"],
        )
        for longitude, latitude in POINTS:
            for resolution in RESOLUTIONS:
                cell = dggs.cell_from_point(
                    resolution, (longitude, latitude), plane=False
                )
                point_indexing.append(
                    {
                        "configuration": configuration["id"],
                        "lonlat": [longitude, latitude],
                        "resolution": resolution,
                        "cell": str(cell),
                    }
                )

        for identifier in GEOMETRY_CELLS:
            cell = dggs.cell(_suid(identifier))
            cell_geometry.append(
                {
                    "configuration": configuration["id"],
                    "cell": identifier,
                    "region": cell.region(),
                    "shape": cell.ellipsoidal_shape,
                    "nucleus_projected": _point(cell.nucleus(plane=True)),
                    "nucleus_lonlat": _point(cell.nucleus(plane=False)),
                    "vertices_projected": _points(cell.vertices(plane=True)),
                    "vertices_lonlat": _points(cell.vertices(plane=False)),
                    "vertices_lonlat_trimmed": _points(
                        cell.vertices(plane=False, trim_dart=True)
                    ),
                    "boundary_projected_n3": _points(
                        cell.boundary(n=3, plane=True)
                    ),
                    "boundary_lonlat_n3": _points(
                        cell.boundary(n=3, plane=False)
                    ),
                    "boundary_projected_interior_n3": _points(
                        cell.boundary(n=3, plane=True, interior=True)
                    ),
                    "boundary_lonlat_interior_n3": _points(
                        cell.boundary(n=3, plane=False, interior=True)
                    ),
                    "neighbors_projected": [
                        [direction, str(neighbor)]
                        for direction, neighbor in cell.neighbors(plane=True).items()
                    ],
                    "neighbors_lonlat": [
                        [direction, str(neighbor)]
                        for direction, neighbor in cell.neighbors(plane=False).items()
                    ],
                }
            )

    canonical = RHEALPixDGGS()
    topology: list[dict[str, Any]] = []
    for identifier in TOPOLOGY_CELLS:
        cell = canonical.cell(_suid(identifier))
        upstream_level = int(cell.index(order="resolution"))
        corrected_level = (
            "NOPQRS".index(identifier) if len(identifier) == 1 else upstream_level
        )
        successor_at: dict[str, str | None] = {}
        successor_errors: dict[str, str] = {}
        predecessor_at: dict[str, str | None] = {}
        for resolution in TRAVERSAL_RESOLUTIONS:
            successor, error = _cell_call(cell, "successor", resolution)
            successor_at[str(resolution)] = successor
            if error is not None:
                successor_errors[str(resolution)] = error
            predecessor_at[str(resolution)] = _cell_call(
                cell, "predecessor", resolution
            )[0]
        topology.append(
            {
                "cell": identifier,
                "level_order_index": corrected_level,
                "upstream_level_order_index": (
                    upstream_level if upstream_level != corrected_level else None
                ),
                "post_order_index": int(cell.index(order="post")),
                "successor": _cell_call(cell, "successor")[0],
                "predecessor": _cell_call(cell, "predecessor")[0],
                "successor_at": successor_at,
                "successor_upstream_errors": successor_errors,
                "predecessor_at": predecessor_at,
                "children": (
                    None
                    if cell.resolution >= canonical.max_resolution
                    else [str(child) for child in cell.subcells()]
                ),
                "children_error": (
                    "maximum_resolution"
                    if cell.resolution >= canonical.max_resolution
                    else None
                ),
            }
        )

    metrics = [
        {
            "resolution": resolution,
            "width_m": float(canonical.cell_width(resolution, plane=True)),
            "area_projected_m2": float(
                canonical.cell_area(resolution, plane=True)
            ),
            "area_ellipsoidal_m2": float(
                canonical.cell_area(resolution, plane=False)
            ),
        }
        for resolution in range(canonical.max_resolution + 1)
    ]

    return {
        "$schema": "schema-v1.json",
        "schema_version": 1,
        "corpus_version": "1.0.0",
        "upstream": {
            "distribution": UPSTREAM_NAME,
            "version": installed_version,
            "source_url": "https://github.com/manaakiwhenua/rhealpixdggs-py",
            "source_files_sha256": {
                name: _sha256(package_root / name) for name in source_files
            },
        },
        "generator": {
            "path": "tools/generate_upstream_corpus.py",
            "dependency_mode": (
                "minimal import stubs for unused Shapely/PROJ surfaces"
                if use_stubs
                else "upstream declared dependencies"
            ),
        },
        "contract": {
            "ellipsoid": "WGS84",
            "aperture": 9,
            "max_resolution": 15,
            "geographic_coordinate_order": "longitude_latitude_degrees",
            "projected_coordinate_unit": "metres",
            "tolerances": {
                "projected_absolute": 1e-6,
                "geographic_absolute": 2e-10,
                "metric_relative": 1e-12,
            },
            "known_corrections": [
                {
                    "id": "resolution_zero_level_indices",
                    "upstream": "Cell.index('resolution') returns 6..11",
                    "expected": "indices are 0..5 and invert correctly",
                },
                {
                    "id": "terminal_finer_successor",
                    "upstream": "Cell('S').successor(finer) raises AttributeError",
                    "expected": "returns null",
                },
            ],
            "doctest_sources": [
                "cell.py::Cell.index",
                "cell.py::Cell.successor",
                "cell.py::Cell.predecessor",
                "cell.py::Cell.nucleus",
                "cell.py::Cell.vertices",
                "cell.py::Cell.boundary",
                "cell.py::Cell.neighbors",
                "dggs.py::RHEALPixDGGS.cell_from_point",
                "dggs.py::RHEALPixDGGS.cell_width",
                "dggs.py::RHEALPixDGGS.cell_area",
            ],
        },
        "counts": {
            "configurations": len(configurations),
            "point_indexing": len(point_indexing),
            "cell_geometry": len(cell_geometry),
            "topology": len(topology),
            "metrics": len(metrics),
        },
        "configurations": configurations,
        "point_indexing": point_indexing,
        "cell_geometry": cell_geometry,
        "topology": topology,
        "mixed_post_order": sorted(
            MIXED_ORDER_CELLS,
            key=lambda identifier: canonical.cell(_suid(identifier)),
        ),
        "metrics": metrics,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--upstream-root",
        type=Path,
        help="directory containing rhealpixdggs and its .dist-info metadata",
    )
    parser.add_argument(
        "--minimal-dependency-stubs",
        action="store_true",
        help="stub unused Shapely and external PROJ imports",
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if the existing output differs from regenerated content",
    )
    arguments = parser.parse_args()
    corpus = build_corpus(
        arguments.upstream_root,
        arguments.minimal_dependency_stubs,
    )
    rendered = json.dumps(corpus, indent=2, allow_nan=False) + "\n"
    checksum_path = arguments.output.with_suffix(".sha256")
    checksum = hashlib.sha256(rendered.encode()).hexdigest()
    checksum_record = f"{checksum}  {arguments.output.name}\n"
    if arguments.check:
        corpus_is_current = (
            arguments.output.exists() and arguments.output.read_text() == rendered
        )
        checksum_is_current = (
            checksum_path.exists()
            and checksum_path.read_text() == checksum_record
        )
        if not corpus_is_current or not checksum_is_current:
            print(f"conformance corpus is stale: {arguments.output}", file=sys.stderr)
            return 1
        print(f"conformance corpus is current: {arguments.output}")
        return 0
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(rendered)
    checksum_path.write_text(checksum_record)
    print(arguments.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
