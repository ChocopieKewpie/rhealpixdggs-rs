import hashlib
import json
from pathlib import Path

import pytest

import rhealpixdggs as rh


UNIT_SQUARE_LATLNG = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
CORPUS_DIR = Path(__file__).parents[1] / "fixtures" / "rhealpixdggs-py-0.6.0"


def test_versioned_coverage_corpus() -> None:
    path = CORPUS_DIR / "coverage-v1.json"
    corpus = json.loads(path.read_text())
    recorded = (CORPUS_DIR / "coverage-v1.sha256").read_text().split()[0]
    assert hashlib.sha256(path.read_bytes()).hexdigest() == recorded
    assert corpus["upstream"]["version"] == "0.6.0"
    assert corpus["counts"] == {
        "point_edges": len(corpus["point_edges"]),
        "regions": len(corpus["regions"]),
        "lines": len(corpus["lines"]),
        "polygons": len(corpus["polygons"]),
    }

    for case in corpus["point_edges"]:
        assert rh.latlng_to_cell(
            case["latitude"], case["longitude"], case["resolution"]
        ) == case["cell"]

    for case in corpus["regions"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        actual = dggs.cells_from_region(
            case["resolution"],
            tuple(case["upper_left"]),
            tuple(case["lower_right"]),
            case["plane"],
        )
        assert [[str(cell) for cell in row] for row in actual] == case["cells"]

    for case in corpus["lines"]:
        dggs = rh.RHEALPixDGGS(
            north_square=case["configuration"][0],
            south_square=case["configuration"][1],
        )
        actual = dggs.cells_from_line(
            case["resolution"],
            tuple(case["start"]),
            tuple(case["end"]),
            case["plane"],
        )
        assert [str(cell) for cell in actual] == case["cells"]

    for case in corpus["polygons"]:
        exterior = [(latitude, longitude) for longitude, latitude in case["exterior"]]
        holes = [
            [(latitude, longitude) for longitude, latitude in ring]
            for ring in case["holes"]
        ]
        assert rh.polygon_to_cells(
            exterior,
            case["resolution"],
            holes=holes,
        ) == case["cells"]


def test_bbox_cover_matches_upstream_region() -> None:
    assert set(rh.bbox_to_cells(60.0, 0.0, 90.0, 0.0, 1)) == {
        "N0",
        "N1",
        "N2",
        "Q0",
        "Q1",
        "Q2",
        "Q3",
        "Q4",
        "Q5",
        "R0",
        "R3",
    }


def test_bbox_cover_splits_at_antimeridian() -> None:
    cells = rh.bbox_to_cells(1.0, -1.0, -179.0, 179.0, 4)
    assert cells == ["O3330", "O3333", "O3336", "R5552", "R5555", "R5558"]


def test_line_cover_matches_upstream_and_preserves_path_order() -> None:
    cells = rh.line_to_cells(
        [(86.549596, -89.669615), (86.0, -134.0)],
        3,
    )
    assert cells == ["N448", "N447"]

    multi_segment = rh.line_to_cells(
        [(86.549596, -89.669615), (86.0, -134.0), (85.8, -145.0)],
        3,
    )
    assert multi_segment[:2] == cells
    assert all(left != right for left, right in zip(multi_segment, multi_segment[1:]))


def test_line_cover_uses_short_antimeridian_path() -> None:
    assert rh.line_to_cells([(0.0, 179.0), (0.0, -179.0)], 3) == [
        "R555",
        "O333",
    ]


def test_line_cover_handles_polar_cap_cells() -> None:
    cells = rh.line_to_cells([(70.0, -170.0), (89.0, 0.0), (70.0, 170.0)], 2)
    assert "N44" in cells
    assert cells[0] == "N70"
    assert cells[-1] == "N38"


def test_polygon_fill_matches_upstream_polyfill_doctests() -> None:
    assert rh.polygon_to_cells(UNIT_SQUARE_LATLNG, 4) == ["Q3330"]
    assert rh.polygon_to_cells(UNIT_SQUARE_LATLNG, 5) == [
        "Q33303",
        "Q33304",
        "Q33305",
        "Q33306",
        "Q33307",
        "Q33308",
        "Q33330",
        "Q33331",
        "Q33332",
    ]


def test_polygon_holes_and_recursive_compaction() -> None:
    hole = [(0.3, 0.3), (0.7, 0.3), (0.7, 0.7), (0.3, 0.7)]
    complete = rh.polygon_to_cells(UNIT_SQUARE_LATLNG, 6)
    with_hole = rh.polygon_to_cells(UNIT_SQUARE_LATLNG, 6, holes=[hole])
    assert set(with_hole) < set(complete)

    compacted = rh.polygon_to_cells(UNIT_SQUARE_LATLNG, 6, compact=True)
    assert {"Q33306", "Q33307", "Q33330", "Q33331"} <= set(compacted)
    assert "Q333060" not in compacted
    assert len(compacted) < len(complete)


def test_polygon_crosses_antimeridian() -> None:
    polygon = [(1.0, 179.0), (1.0, -179.0), (-1.0, -179.0), (-1.0, 179.0)]
    cells = rh.polygon_to_cells(polygon, 4)
    assert cells
    assert {rh.get_base_cell_number(cell) for cell in cells} == {1, 4}


def test_polygon_intersection_cover_includes_tiny_nz_fragment() -> None:
    target = rh.latlng_to_cell(-40.0, 175.0, 8)
    polygon = [
        (-40.0, 175.0),
        (-40.0, 175.000_000_1),
        (-39.999_999_9, 175.000_000_1),
        (-39.999_999_9, 175.0),
    ]
    assert rh.polygon_to_cells(polygon, 8) == []
    cells = rh.polygon_to_cells_intersects(polygon, 8)
    assert target in cells
    assert rh.polygon_to_cells_intersects(list(reversed(polygon)), 8) == cells


def test_polygon_intersection_cover_is_a_strict_superset_for_unit_square() -> None:
    centroid = set(rh.polygon_to_cells(UNIT_SQUARE_LATLNG, 5))
    intersects = set(rh.polygon_to_cells_intersects(UNIT_SQUARE_LATLNG, 5))
    assert centroid < intersects


def test_polygon_intersection_cover_handles_compaction_antimeridian_and_caps() -> None:
    raw = rh.polygon_to_cells_intersects(UNIT_SQUARE_LATLNG, 6)
    compacted = rh.polygon_to_cells_intersects(
        UNIT_SQUARE_LATLNG, 6, compact=True
    )
    assert len(compacted) < len(raw)
    assert set(rh.uncompact_cells(compacted, 6)) == set(raw)

    antimeridian = [(1.0, 179.0), (1.0, -179.0), (-1.0, -179.0), (-1.0, 179.0)]
    cells = rh.polygon_to_cells_intersects(antimeridian, 4)
    assert {rh.get_base_cell_number(cell) for cell in cells} == {1, 4}

    north_pole = [(89.0, -1.0), (90.0, -1.0), (90.0, 1.0), (89.0, 1.0)]
    south_pole = [(-89.0, -1.0), (-90.0, -1.0), (-90.0, 1.0), (-89.0, 1.0)]
    assert "N4444" in rh.polygon_to_cells_intersects(north_pole, 4)
    assert "S4444" in rh.polygon_to_cells_intersects(south_pole, 4)


def test_coverage_validation() -> None:
    with pytest.raises(ValueError, match="at least two"):
        rh.line_to_cells([(0.0, 0.0)], 3)
    with pytest.raises(ValueError, match="at least three"):
        rh.polygon_to_cells([(0.0, 0.0), (1.0, 1.0)], 3)
    with pytest.raises(ValueError, match="zero area"):
        rh.polygon_to_cells([(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)], 3)
    step = 2.0**-20
    with pytest.raises(ValueError, match="zero area"):
        rh.polygon_to_cells(
            [
                (-40.0, 175.0),
                (-40.0 + step, 175.0 + step),
                (-40.0 + 2.0 * step, 175.0 + 2.0 * step),
            ],
            8,
        )
    with pytest.raises(ValueError, match="latitude"):
        rh.bbox_to_cells(91.0, 0.0, 1.0, 0.0, 3)


def test_polygon_validation_accepts_tiny_nz_fragments() -> None:
    tiny = [
        (-40.0, 175.0),
        (-40.0, 175.000_000_1),
        (-39.999_999_9, 175.000_000_1),
        (-39.999_999_9, 175.0),
    ]
    sliver = [
        (-40.0, 175.0),
        (-40.0, 175.001),
        (-39.999_999_999_9, 175.001),
        (-39.999_999_999_9, 175.0),
    ]

    for exterior in (tiny, sliver):
        cells = rh.polygon_to_cells(exterior, 8)
        assert rh.polygon_to_cells(list(reversed(exterior)), 8) == cells


def test_polygon_validation_is_location_and_scale_invariant() -> None:
    locations = [
        (-80.0, -179.0),
        (45.0, -120.0),
        (0.0, 0.0),
        (-45.0, 120.0),
        (80.0, 179.0),
    ]

    for exponent in (10, 20, 30, 40):
        width = 2.0**-exponent
        height = 2.0 ** (-exponent - 2)
        for latitude, longitude in locations:
            exterior = [
                (latitude, longitude),
                (latitude, longitude + width),
                (latitude + height, longitude + width),
                (latitude + height, longitude),
            ]
            cells = rh.polygon_to_cells(exterior, 8)
            assert rh.polygon_to_cells(list(reversed(exterior)), 8) == cells


def test_polygon_validation_accepts_tiny_antimeridian_and_polar_rings() -> None:
    step = 2.0**-20
    cases = [
        [
            (-10.0, 180.0 - step),
            (-10.0, -180.0 + step),
            (-10.0 + step, -180.0 + step),
            (-10.0 + step, 180.0 - step),
        ],
        [
            (90.0 - 4.0 * step, 45.0),
            (90.0 - 4.0 * step, 45.0 + step),
            (90.0 - 3.0 * step, 45.0 + step),
            (90.0 - 3.0 * step, 45.0),
        ],
        [
            (-90.0 + 3.0 * step, -45.0),
            (-90.0 + 3.0 * step, -45.0 + step),
            (-90.0 + 4.0 * step, -45.0 + step),
            (-90.0 + 4.0 * step, -45.0),
        ],
    ]

    for exterior in cases:
        cells = rh.polygon_to_cells(exterior, 8)
        assert rh.polygon_to_cells(list(reversed(exterior)), 8) == cells


def test_upstream_facade_region_line_and_centroid() -> None:
    dggs = rh.RHEALPixDGGS()
    rows = dggs.cells_from_region(1, (0.0, 60.0), (90.0, 0.0), plane=False)
    assert [[str(cell) for cell in row] for row in rows] == [
        ["N2", "N1", "N0"],
        ["Q0", "Q1", "Q2", "R0"],
        ["Q3", "Q4", "Q5", "R3"],
    ]
    assert [
        str(cell)
        for cell in dggs.cells_from_line(
            3,
            (-89.669615, 86.549596),
            (-134.0, 86.0),
            plane=False,
        )
    ] == ["N448", "N447"]

    cell = dggs.cell("N0")
    assert cell.centroid() == cell.nucleus()
    longitude, latitude = cell.centroid(plane=False)
    assert longitude == pytest.approx(90.0, abs=2e-10)
    assert 41.9 < latitude < 74.5


def test_quad_centroid_uses_mean_latitude_not_edge_latitude_midpoint() -> None:
    dggs = rh.RHEALPixDGGS()
    for identifier, expected_latitude in [
        ("Q7", -26.790327),
        ("O0", 26.790327),
        ("P31", 8.565250),
    ]:
        cell = dggs.cell(identifier)
        longitude, latitude = cell.centroid(plane=False)
        assert longitude == pytest.approx(cell.nucleus(plane=False)[0], abs=2e-10)
        assert latitude == pytest.approx(expected_latitude, abs=1e-6)
