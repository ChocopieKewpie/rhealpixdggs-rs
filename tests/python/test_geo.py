from __future__ import annotations

from pathlib import Path

import pytest

import rhealpixdggs as rh
from rhealpixdggs import geo


class Ring:
    def __init__(self, coordinates):
        self.coords = coordinates


class Polygon:
    geom_type = "Polygon"
    is_empty = False
    is_valid = True

    def __init__(self, exterior, holes=()):
        self.exterior = Ring(exterior)
        self.interiors = [Ring(hole) for hole in holes]


class MultiPolygon:
    geom_type = "MultiPolygon"
    is_empty = False
    is_valid = True

    def __init__(self, polygons):
        self.geoms = polygons


UNIT_SQUARE = Polygon(
    [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)]
)


def test_geometry_adapter_matches_coordinate_api() -> None:
    expected = rh.polygon_to_cells(
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        5,
    )
    assert geo.geometry_to_cells(UNIT_SQUARE, 5) == expected


def test_geometry_adapter_supports_intersection_coverage() -> None:
    expected = rh.polygon_to_cells_intersects(
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        5,
    )
    assert geo.geometry_to_cells(
        UNIT_SQUARE, 5, coverage_mode="intersects"
    ) == expected
    with pytest.raises(ValueError, match="coverage_mode"):
        geo.geometry_to_cells(UNIT_SQUARE, 5, coverage_mode="anything")


def test_cli_accepts_intersection_coverage_mode() -> None:
    from rhealpixdggs.cli import parser

    arguments = parser().parse_args(
        ["input.geojson", "output.gpkg", "-r", "6", "--coverage-mode", "intersects"]
    )
    assert arguments.coverage_mode == "intersects"


def test_multipolygon_deduplicates_and_compacts_after_union() -> None:
    duplicated = MultiPolygon([UNIT_SQUARE, UNIT_SQUARE])
    assert geo.geometry_to_cells(duplicated, 6) == geo.geometry_to_cells(
        UNIT_SQUARE, 6
    )
    assert geo.geometry_to_cells(duplicated, 6, compact=True) == rh.compact_cells(
        geo.geometry_to_cells(UNIT_SQUARE, 6)
    )


def test_geometry_adapter_validates_polygonal_input() -> None:
    invalid = Polygon([])
    invalid.is_valid = False
    with pytest.raises(ValueError, match="valid"):
        geo.geometry_to_cells(invalid, 4)

    line = Polygon([])
    line.geom_type = "LineString"
    with pytest.raises(TypeError, match="Polygon"):
        geo.geometry_to_cells(line, 4)


def test_geometry_adapter_accepts_tiny_bisection_fragments() -> None:
    tiny = Polygon(
        [
            (175.0, -40.0),
            (175.000_000_1, -40.0),
            (175.000_000_1, -39.999_999_9),
            (175.0, -39.999_999_9),
            (175.0, -40.0),
        ]
    )
    sliver = Polygon(
        [
            (175.0, -40.0),
            (175.001, -40.0),
            (175.001, -39.999_999_999_9),
            (175.0, -39.999_999_999_9),
            (175.0, -40.0),
        ]
    )

    for geometry in (tiny, sliver):
        assert isinstance(geo.geometry_to_cells(geometry, 8), list)


def test_geopackage_round_trip_when_geo_extra_is_installed(tmp_path: Path) -> None:
    geopandas = pytest.importorskip("geopandas")
    pytest.importorskip("pyogrio")
    pytest.importorskip("shapely")

    source = (
        Path(__file__).parents[2]
        / "benchmarks"
        / "data"
        / "new-zealand-simplified.geojson"
    )
    output = tmp_path / "new-zealand-rhealpix.gpkg"
    frame = geo.polygon_file_to_geopackage(
        source, output, 4, coverage_mode="intersects"
    )

    assert output.exists()
    assert not frame.empty
    assert frame.crs.to_epsg() == 4326
    assert set(frame.geometry.geom_type) == {"MultiPolygon"}
    restored = geopandas.read_file(output, layer="rhealpix_cells")
    assert restored["cell_id"].tolist() == frame["cell_id"].tolist()
    centroid = geo.polygon_file_to_geopackage(
        source, tmp_path / "new-zealand-centroid.gpkg", 4
    )
    assert set(centroid["cell_id"]) <= set(frame["cell_id"])
