import hashlib
import json
from pathlib import Path

import pytest

import rhealpixdggs as rh


CORPUS_DIR = Path(__file__).parents[1] / "fixtures" / "rhealpixdggs-py-0.6.0"


def _assert_point(
    actual: tuple[float, float],
    expected: list[float],
    tolerance: float,
    *,
    geographic: bool = False,
) -> None:
    if geographic:
        longitude_delta = (actual[0] - expected[0] + 180.0) % 360.0 - 180.0
        assert longitude_delta == pytest.approx(0.0, abs=tolerance)
    else:
        assert actual[0] == pytest.approx(expected[0], abs=tolerance)
    assert actual[1] == pytest.approx(expected[1], abs=tolerance)


def _assert_points(
    actual: list[tuple[float, float]],
    expected: list[list[float]],
    tolerance: float,
    *,
    geographic: bool = False,
) -> None:
    assert len(actual) == len(expected)
    for actual_point, expected_point in zip(actual, expected):
        _assert_point(
            actual_point,
            expected_point,
            tolerance,
            geographic=geographic,
        )


def _assert_triple(
    actual: tuple[float, float, float], expected: list[float], tolerance: float
) -> None:
    assert actual == pytest.approx(expected, abs=tolerance)


def test_versioned_facade_corpus() -> None:
    path = CORPUS_DIR / "facade-v1.json"
    corpus = json.loads(path.read_text())
    recorded = (CORPUS_DIR / "facade-v1.sha256").read_text().split()[0]
    assert hashlib.sha256(path.read_bytes()).hexdigest() == recorded
    assert corpus["upstream"]["version"] == "0.6.0"
    projected_tolerance = corpus["error_budget"]["projected_absolute_metres"]
    geographic_tolerance = corpus["error_budget"][
        "geographic_absolute_degrees"
    ]

    for case in corpus["projections"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        function = getattr(dggs, case["projection"])
        projected = function(*case["lonlat"])
        _assert_point(projected, case["projected"], projected_tolerance)
        _assert_point(
            function(*case["projected"], inverse=True),
            case["roundtrip"],
            geographic_tolerance,
            geographic=True,
        )

    for case in corpus["triangle_transforms"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        transformed = dggs.combine_triangles(*case["healpix"])
        _assert_point(transformed, case["rhealpix"], projected_tolerance)
        _assert_point(
            dggs.combine_triangles(*case["rhealpix"], inverse=True),
            case["roundtrip"],
            projected_tolerance,
        )

    for case in corpus["triangles"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        assert dggs.triangle(*case["point"], inverse=case["inverse"]) == (
            case["number"],
            case["region"],
        )

    for case in corpus["cartesian"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        _assert_triple(
            dggs.xyz(*case["lonlat"], lonlat=True),
            case["xyz_lonlat"],
            projected_tolerance,
        )
        _assert_triple(
            dggs.xyz(*case["projected"]),
            case["xyz_projected"],
            projected_tolerance,
        )
        _assert_triple(
            dggs.xyz_cube(*case["lonlat"], lonlat=True),
            case["cube_lonlat"],
            projected_tolerance,
        )
        _assert_triple(
            dggs.xyz_cube(*case["projected"]),
            case["cube_projected"],
            projected_tolerance,
        )
    for case in corpus["region_parents"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        cell = dggs.cell_from_region(
            tuple(case["upper_left"]),
            tuple(case["lower_right"]),
            plane=case["plane"],
        )
        assert (None if cell is None else str(cell)) == case["cell"]

    for case in corpus["cell_latitudes"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        assert dggs.cell_latitudes(
            case["resolution"],
            case["minimum"],
            case["maximum"],
            nucleus=case["nucleus"],
            plane=case["plane"],
        ) == pytest.approx(case["values"], abs=geographic_tolerance)

    for key, method in [
        ("meridians", "cells_from_meridian"),
        ("parallels", "cells_from_parallel"),
    ]:
        for case in corpus[key]:
            dggs = rh.RHEALPixDGGS(
                north_square=case["configuration"][0],
                south_square=case["configuration"][1],
            )
            if key == "meridians":
                cells = getattr(dggs, method)(
                    case["resolution"],
                    case["longitude"],
                    case["latitude_min"],
                    case["latitude_max"],
                )
            else:
                cells = getattr(dggs, method)(
                    case["resolution"],
                    case["latitude"],
                    case["longitude_min"],
                    case["longitude_max"],
                )
            assert [str(cell) for cell in cells] == case["cells"]

    canonical = rh.RHEALPixDGGS()
    for case in corpus["minimal_covers"]:
        assert [
            str(cell)
            for cell in canonical.minimal_cover(
                case["resolution"],
                [tuple(point) for point in case["points"]],
                plane=case["plane"],
            )
        ] == case["cells"]
    for case in corpus["intervals"]:
        assert [
            str(cell)
            for cell in canonical.interval(
                canonical.cell(case["start"]), canonical.cell(case["end"])
            )
        ] == case["cells"]

    for case in corpus["cells"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        cell = dggs.cell(case["cell"])
        rows, columns = cell.suid_rowcol()
        assert list(rows) == case["row_suid"]
        assert list(columns) == case["column_suid"]
        assert [str(cell.rotate(turns)) for turns in range(4)] == case["rotations"]
        _assert_point(
            cell.ul_vertex(), case["upper_left_projected"], projected_tolerance
        )
        _assert_point(
            cell.ul_vertex(plane=False),
            case["upper_left_lonlat"],
            geographic_tolerance,
            geographic=True,
        )
        _assert_point(
            cell.nw_vertex(), case["northwest_projected"], projected_tolerance
        )
        _assert_point(
            cell.nw_vertex(plane=False),
            case["northwest_lonlat"],
            geographic_tolerance,
            geographic=True,
        )
        actual_ranges = cell.xy_range()
        assert len(actual_ranges) == len(case["xy_range"])
        for actual_range, expected_range in zip(
            actual_ranges, case["xy_range"]
        ):
            assert actual_range == pytest.approx(
                expected_range, abs=projected_tolerance
            )
        interior = cell.interior(n=3)
        assert len(interior) == len(case["interior_projected_n3"])
        for actual_row, expected_row in zip(
            interior, case["interior_projected_n3"]
        ):
            _assert_points(actual_row, expected_row, projected_tolerance)
        _assert_points(
            cell.interior(n=3, plane=False, flatten=True),
            case["interior_lonlat_flat_n3"],
            geographic_tolerance,
            geographic=True,
        )
        assert cell.contains(cell.nucleus()) == case["contains_projected_nucleus"]
        assert cell.contains(cell.nucleus(plane=False), plane=False) == case[
            "contains_lonlat_nucleus"
        ]
        assert {
            str(value): cell.intersects_meridian(value)
            for value in [-180.0, -90.0, 0.0, 90.0, 180.0]
        } == case["meridians"]
        assert {
            str(value): cell.intersects_parallel(value)
            for value in [-90.0, -60.0, 0.0, 60.0, 90.0]
        } == case["parallels"]

    for case in corpus["overlaps"]:
        left = canonical.cell(case["left"])
        right = canonical.cell(case["right"])
        assert left.overlaps(right) == case["overlaps"]
        assert left.subcell(right) == case["left"].startswith(case["right"])
        assert left.region_overlaps([right]) == case["overlaps"]


def test_facade_support_helpers_and_validation() -> None:
    dggs = rh.RHEALPixDGGS()
    assert dggs.antimeridian_check_and_flip(
        [(180.0, 1.0), (-170.0, 0.0)], plane=False
    ) == [(-180.0, 1.0), (-170.0, 0.0)]
    assert set(dggs.area_error_budget()) == set(range(rh.MAX_RESOLUTION + 1))
    with pytest.raises(ValueError, match="at least 2"):
        dggs.cell("N").interior(n=1)
