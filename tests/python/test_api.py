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

