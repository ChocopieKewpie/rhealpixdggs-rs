import hashlib
import json
import math
from pathlib import Path
from typing import Any

import pytest

import rhealpixdggs as rh

CORPUS_DIR = (
    Path(__file__).parents[1] / "fixtures" / "rhealpixdggs-py-0.6.0"
)
CORPUS_PATH = CORPUS_DIR / "conformance-v1.json"


@pytest.fixture(scope="module")
def corpus() -> dict[str, Any]:
    return json.loads(CORPUS_PATH.read_text())


def _configuration_map(corpus: dict[str, Any]) -> dict[str, rh.RHEALPixDGGS]:
    return {
        item["id"]: rh.RHEALPixDGGS(
            north_square=item["north_square"],
            south_square=item["south_square"],
        )
        for item in corpus["configurations"]
    }


def _assert_close(
    actual: tuple[float, float],
    expected: list[float],
    tolerance: float,
    *,
    geographic: bool = False,
) -> None:
    difference_x = actual[0] - expected[0]
    if geographic:
        difference_x = (difference_x + 180.0) % 360.0 - 180.0
    assert abs(difference_x) <= tolerance
    assert abs(actual[1] - expected[1]) <= tolerance


def _assert_points_close(
    actual: list[tuple[float, float]],
    expected: list[list[float]],
    tolerance: float,
    *,
    geographic: bool = False,
) -> None:
    assert len(actual) == len(expected)
    for point, reference in zip(actual, expected):
        _assert_close(point, reference, tolerance, geographic=geographic)


def test_corpus_provenance_counts_and_checksum(corpus: dict[str, Any]) -> None:
    assert corpus["schema_version"] == 1
    assert corpus["corpus_version"] == "1.0.0"
    assert corpus["upstream"]["distribution"] == "rHEALPixDGGS"
    assert corpus["upstream"]["version"] == "0.6.0"
    assert len(corpus["upstream"]["source_files_sha256"]) == 7
    for section in (
        "configurations",
        "point_indexing",
        "cell_geometry",
        "topology",
        "metrics",
    ):
        assert corpus["counts"][section] == len(corpus[section])

    recorded = (CORPUS_DIR / "conformance-v1.sha256").read_text().split()[0]
    actual = hashlib.sha256(CORPUS_PATH.read_bytes()).hexdigest()
    assert actual == recorded


def test_point_indexing_corpus(corpus: dict[str, Any]) -> None:
    configurations = _configuration_map(corpus)
    for case in corpus["point_indexing"]:
        longitude, latitude = case["lonlat"]
        cell = configurations[case["configuration"]].cell_from_point(
            case["resolution"],
            (longitude, latitude),
            plane=False,
        )
        assert str(cell) == case["cell"]


def test_geometry_corpus(corpus: dict[str, Any]) -> None:
    configurations = _configuration_map(corpus)
    tolerances = corpus["contract"]["tolerances"]
    projected_tolerance = tolerances["projected_absolute"]
    geographic_tolerance = tolerances["geographic_absolute"]
    for case in corpus["cell_geometry"]:
        cell = configurations[case["configuration"]].cell(case["cell"])
        assert cell.region() == case["region"]
        assert cell.ellipsoidal_shape == case["shape"]
        _assert_close(
            cell.nucleus(plane=True),
            case["nucleus_projected"],
            projected_tolerance,
        )
        _assert_close(
            cell.nucleus(plane=False),
            case["nucleus_lonlat"],
            geographic_tolerance,
            geographic=True,
        )
        _assert_points_close(
            cell.vertices(plane=True),
            case["vertices_projected"],
            projected_tolerance,
        )
        _assert_points_close(
            cell.vertices(plane=False),
            case["vertices_lonlat"],
            geographic_tolerance,
            geographic=True,
        )
        _assert_points_close(
            cell.vertices(plane=False, trim_dart=True),
            case["vertices_lonlat_trimmed"],
            geographic_tolerance,
            geographic=True,
        )
        _assert_points_close(
            cell.boundary(n=3, plane=True),
            case["boundary_projected_n3"],
            projected_tolerance,
        )
        boundary_lonlat = cell.boundary(n=3, plane=False)
        if case["shape"] in {"quad", "cap"}:
            _assert_points_close(
                boundary_lonlat[::2],
                case["boundary_lonlat_n3"],
                geographic_tolerance,
                geographic=True,
            )
        else:
            _assert_points_close(
                boundary_lonlat,
                case["boundary_lonlat_n3"],
                geographic_tolerance,
                geographic=True,
            )
        _assert_points_close(
            cell.boundary(n=3, plane=True, interior=True),
            case["boundary_projected_interior_n3"],
            projected_tolerance,
        )
        inset_boundary_lonlat = cell.boundary(n=3, plane=False, interior=True)
        if case["shape"] in {"quad", "cap"}:
            assert len(inset_boundary_lonlat) == 8
            assert len(case["boundary_lonlat_interior_n3"]) == 4
        else:
            _assert_points_close(
                inset_boundary_lonlat,
                case["boundary_lonlat_interior_n3"],
                geographic_tolerance,
                geographic=True,
            )
        assert [
            [direction, str(neighbor)]
            for direction, neighbor in cell.neighbors(plane=True).items()
        ] == case["neighbors_projected"]
        assert [
            [direction, str(neighbor)]
            for direction, neighbor in cell.neighbors(plane=False).items()
        ] == case["neighbors_lonlat"]


def test_topology_corpus(corpus: dict[str, Any]) -> None:
    dggs = rh.RHEALPixDGGS()
    for case in corpus["topology"]:
        cell = dggs.cell(case["cell"])
        assert cell.index() == case["level_order_index"]
        assert cell.index("post") == case["post_order_index"]
        assert dggs.cell(level_order_index=cell.index()) == cell
        assert dggs.cell(post_order_index=cell.index("post")) == cell
        successor = cell.successor()
        predecessor = cell.predecessor()
        assert (None if successor is None else str(successor)) == case["successor"]
        assert (None if predecessor is None else str(predecessor)) == case[
            "predecessor"
        ]
        for resolution, expected in case["successor_at"].items():
            successor = cell.successor(int(resolution))
            assert (None if successor is None else str(successor)) == expected
        for resolution, expected in case["predecessor_at"].items():
            predecessor = cell.predecessor(int(resolution))
            assert (None if predecessor is None else str(predecessor)) == expected
        if case["children_error"] == "maximum_resolution":
            with pytest.raises(ValueError):
                list(cell.subcells())
        else:
            assert [str(child) for child in cell.subcells()] == case["children"]

    ordered = sorted(dggs.cell(identifier) for identifier in corpus["mixed_post_order"])
    assert [str(cell) for cell in ordered] == corpus["mixed_post_order"]


def test_metric_corpus(corpus: dict[str, Any]) -> None:
    dggs = rh.RHEALPixDGGS()
    relative_tolerance = corpus["contract"]["tolerances"]["metric_relative"]
    for case in corpus["metrics"]:
        resolution = case["resolution"]
        assert math.isclose(
            dggs.cell_width(resolution),
            case["width_m"],
            rel_tol=relative_tolerance,
        )
        assert math.isclose(
            dggs.cell_area(resolution, plane=True),
            case["area_projected_m2"],
            rel_tol=relative_tolerance,
        )
        assert math.isclose(
            dggs.cell_area(resolution, plane=False),
            case["area_ellipsoidal_m2"],
            rel_tol=relative_tolerance,
        )
