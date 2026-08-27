import math

import pytest

import rhealpixdggs as rh


@pytest.mark.parametrize(
    ("latitude", "longitude", "resolution", "expected"),
    [
        (0.0, 0.0, 0, "Q"),
        (0.0, 0.0, 1, "Q3"),
        (-36.8485, 174.7633, 8, "R88446545"),
        (-40.356, 175.611, 12, "R887560473610"),
        (37.7749, -122.4194, 15, "O125051437212863"),
        (72.5, -179.999, 12, "N622446670001"),
    ],
)
def test_matches_upstream_wgs84_003_golden_cells(
    latitude: float, longitude: float, resolution: int, expected: str
) -> None:
    assert rh.latlng_to_cell(latitude, longitude, resolution) == expected


def test_cell_nucleus_round_trip() -> None:
    cell = rh.latlng_to_cell(-40.356, 175.611, 12)
    latitude, longitude = rh.cell_to_latlng(cell)
    assert rh.latlng_to_cell(latitude, longitude, 12) == cell


def test_hierarchy_and_compaction() -> None:
    children = rh.cell_to_children("P")
    assert children == [f"P{digit}" for digit in range(9)]
    assert rh.compact_cells(children) == ["P"]
    assert rh.uncompact_cells(["P"], 1) == children
    assert rh.cell_to_parent("P8") == "P"


def test_integer_round_trip() -> None:
    cell = "S444375206675068"
    assert rh.int_to_str(rh.str_to_int(cell)) == cell


def test_area() -> None:
    area = rh.cell_area("Q")
    assert math.isclose(area, 85_010_936_954_014.78, abs_tol=0.1)
    assert math.isclose(rh.cell_area("Q0", "km2"), area / 9 / 1e6)


def test_validation() -> None:
    assert rh.is_valid_cell("N038")
    assert not rh.is_valid_cell("N9")
    with pytest.raises(ValueError):
        rh.cell_to_latlng("not-a-cell")
    with pytest.raises(ValueError):
        rh.latlng_to_cell(91.0, 0.0, 3)


@pytest.mark.parametrize(
    ("cell", "region", "shape"),
    [
        ("P2", "equatorial", "quad"),
        ("N", "north_polar", "cap"),
        ("S4", "south_polar", "cap"),
        ("N62", "north_polar", "dart"),
        ("S43", "south_polar", "skew_quad"),
    ],
)
def test_region_and_shape_match_upstream(cell: str, region: str, shape: str) -> None:
    assert rh.get_cell_region(cell) == region
    assert rh.get_cell_shape(cell) == shape


def test_planar_neighbors_include_polar_rotations() -> None:
    assert rh.cell_to_neighbors("N0") == {
        "left": "R0",
        "right": "N1",
        "down": "N3",
        "up": "Q2",
    }
    assert rh.cell_to_neighbor("Q888", "right") == "R666"
    assert rh.cell_to_neighbor("Q888", "down") == "S666"
    with pytest.raises(ValueError):
        rh.cell_to_neighbor("N0", "north")


@pytest.mark.parametrize(
    ("cell", "expected"),
    [
        (
            "P2",
            {"north": "N2", "south": "P5", "west": "P1", "east": "Q0"},
        ),
        (
            "N",
            {"south_0": "O", "south_1": "P", "south_2": "Q", "south_3": "R"},
        ),
        (
            "N0",
            {
                "west": "N1",
                "south_west": "Q2",
                "south_east": "R0",
                "east": "N3",
            },
        ),
        (
            "S43",
            {"north": "S35", "south": "S44", "east": "S40", "west": "S46"},
        ),
    ],
)
def test_ellipsoidal_neighbors_match_upstream(
    cell: str, expected: dict[str, str]
) -> None:
    assert rh.cell_to_neighbors(cell, plane=False) == expected
    for direction, neighbour in expected.items():
        assert rh.cell_to_neighbor(cell, direction, plane=False) == neighbour


def test_ellipsoidal_direction_validation() -> None:
    assert rh.cell_to_neighbor("P2", "north_west", plane=False) is None
    with pytest.raises(ValueError):
        rh.cell_to_neighbor("P2", "left", plane=False)


def test_geographic_vertex_order_and_dart_trimming() -> None:
    boundary = rh.cell_to_boundary("N0")
    assert len(boundary) == 4
    expected = [
        (74.424006701996, 90.0),
        (41.937853910160, 120.0),
        (41.937853910160, 90.0),
        (41.937853910160, 60.0),
    ]
    for actual, reference in zip(boundary, expected):
        assert actual == pytest.approx(reference, abs=2e-10)
    assert len(rh.cell_to_boundary("N0", trim_dart=True)) == 3
    assert len(rh.cell_to_boundary("N43", trim_dart=True)) == 4


def test_upstream_object_facade() -> None:
    dggs = rh.RHEALPixDGGS()
    cell = dggs.cell(("N", 6, 2))
    assert isinstance(cell, rh.Cell)
    assert str(cell) == "N62"
    assert cell.suid == ("N", 6, 2)
    assert cell.resolution == 2
    assert cell.region() == "north_polar"
    assert cell.ellipsoidal_shape == "dart"
    assert {name: str(value) for name, value in cell.neighbors().items()} == {
        "left": "N61",
        "right": "N70",
        "down": "N65",
        "up": "N38",
    }
    assert len(cell.vertices(plane=False, trim_dart=True)) == 3
    assert [str(value) for value in dggs.cell("P").subcells()] == [
        f"P{digit}" for digit in range(9)
    ]
    assert str(dggs.cell_from_point(1, (0.0, 0.0))) == "Q3"
    assert str(dggs.cell_from_point(1, (0.0, 45.0), plane=False)) == "N2"


def test_facade_respects_custom_polar_square_positions_and_metrics() -> None:
    dggs = rh.RHEALPixDGGS(north_square=1, south_square=3)
    assert str(dggs.cell("N0").neighbor("left")) == "O0"
    assert str(dggs.cell("S0").neighbor("up")) == "R6"
    cell = dggs.cell("P57")
    assert math.isclose(cell.area(plane=True), cell.width() ** 2)
    assert math.isclose(cell.area(plane=False), rh.cell_area("P57"))
    assert cell.width(plane=False) is None
    ellipsoidal_neighbors = dggs.cell("N0").neighbors(plane=False)
    assert list(ellipsoidal_neighbors) == [
        "west",
        "south_west",
        "south_east",
        "east",
    ]
    assert {
        direction: str(neighbour)
        for direction, neighbour in ellipsoidal_neighbors.items()
    } == {
        "west": "N1",
        "south_west": "R2",
        "south_east": "O0",
        "east": "N3",
    }
    assert str(dggs.cell("S0").neighbor("north_east", plane=False)) == "R6"
    assert list(rh.WGS84_003.cell("N").neighbors(plane=False)) == [
        "south_0",
        "south_1",
        "south_2",
        "south_3",
    ]
