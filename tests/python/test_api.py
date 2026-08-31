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


def test_level_and_post_order_index_round_trips() -> None:
    cases = [
        ("N", 0, 231_627_523_606_479),
        ("N2", 8, 77_209_174_535_492),
        ("N82", 134, 214_469_929_265_257),
        ("Q381", 3_049, 795_604_004_266_974),
        ("S", 5, 1_389_765_141_638_879),
    ]
    for cell, level, post in cases:
        assert rh.cell_to_level_order_index(cell) == level
        assert rh.level_order_index_to_cell(level) == cell
        assert rh.cell_to_post_order_index(cell) == post
        assert rh.post_order_index_to_cell(post) == cell

    with pytest.raises(ValueError, match="level-order"):
        rh.level_order_index_to_cell(1_389_765_141_638_880)
    with pytest.raises(ValueError, match="post-order"):
        rh.post_order_index_to_cell(1_389_765_141_638_880)


def test_predecessor_and_successor_traversal() -> None:
    assert rh.cell_to_successor("N82") == "N83"
    assert rh.cell_to_successor("N82", 0) == "O"
    assert rh.cell_to_successor("N82", 1) == "O0"
    assert rh.cell_to_successor("N82", 3) == "N830"
    assert rh.cell_to_predecessor("N08") == "N07"
    assert rh.cell_to_predecessor("N08", 0) is None
    assert rh.cell_to_predecessor("N08", 3) == "N088"
    assert rh.cell_to_successor("S", 15) is None

    with pytest.raises(ValueError, match="resolution"):
        rh.cell_to_successor("N82", 16)


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


def test_grid_disk_and_ring_use_shortest_edge_distance() -> None:
    assert rh.grid_ring("Q44", 0) == ["Q44"]
    assert rh.grid_disk("Q44", 0) == ["Q44"]

    ring_one = rh.grid_ring("Q44", 1)
    ring_two = rh.grid_ring("Q44", 2)
    disk_two = rh.grid_disk("Q44", 2)
    assert ring_one == ["Q41", "Q43", "Q45", "Q47"]
    assert len(ring_two) == 8
    assert len(disk_two) == 13
    assert disk_two == ["Q44", *ring_one, *ring_two]


def test_grid_topology_crosses_global_face_seams() -> None:
    assert "O666" in rh.grid_ring("R888", 1)
    assert "S666" in rh.grid_ring("Q888", 1)
    assert rh.grid_ring("N0", 1) == ["N1", "N3", "Q2", "R0"]

    for origin in ("N", "S", "N0", "S43", "R888"):
        for neighbour in rh.grid_ring(origin, 1):
            assert rh.are_neighbor_cells(origin, neighbour)
            assert rh.are_neighbor_cells(neighbour, origin)
        assert not rh.are_neighbor_cells(origin, origin)


def test_grid_topology_validates_resolution_and_expansion() -> None:
    with pytest.raises(ValueError, match="non-negative"):
        rh.grid_ring("Q44", -1)
    with pytest.raises(ValueError, match="same resolution"):
        rh.are_neighbor_cells("Q4", "Q44")
    with pytest.raises(ValueError, match="distance 3000 exceeds the maximum 2235"):
        rh.grid_disk("Q44444444", 3000)


def test_de9im_cell_predicates_cover_hierarchy_edges_corners_and_disjointness() -> None:
    assert rh.cell_equals("Q4", "Q4")
    assert not rh.cell_equals("Q4", "Q44")
    assert rh.cell_contains("Q4", "Q44")
    assert rh.cell_covers("Q4", "Q44")
    assert rh.cell_within("Q44", "Q4")
    assert rh.cell_covered_by("Q44", "Q4")

    for left, right in [("Q4", "Q5"), ("Q4", "Q8"), ("Q0", "Q40"), ("N", "O")]:
        assert rh.cell_touches(left, right)
        assert rh.cell_touches(right, left)
        assert rh.cell_intersects(left, right)
        assert not rh.cell_disjoint(left, right)

    assert rh.cell_disjoint("N", "S")
    assert not rh.cell_intersects("N", "S")
    assert not rh.cell_crosses("Q4", "Q5")
    assert not rh.cell_overlaps("Q4", "Q44")


def test_object_facade_exposes_topological_predicates_without_changing_legacy_overlap() -> None:
    dggs = rh.RHEALPixDGGS(north_square=2, south_square=1)
    parent = dggs.cell("Q4")
    child = dggs.cell("Q44")
    edge = dggs.cell("Q5")
    corner = dggs.cell("Q8")
    assert parent.equals(dggs.cell("Q4"))
    assert parent.contains(child)
    assert child.within(parent)
    assert parent.covers(child)
    assert child.covered_by(parent)
    assert parent.touches(edge)
    assert parent.touches(corner)
    assert parent.intersects(child)
    assert not parent.disjoint(child)
    assert not parent.crosses(edge)
    assert not parent.topologically_overlaps(child)
    assert parent.overlaps(child)  # retained upstream hierarchical meaning

    with pytest.raises(ValueError, match="same DGGS"):
        parent.touches(rh.RHEALPixDGGS().cell("Q5"))


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


def test_densified_boundary_has_an_exact_point_count_for_every_shape() -> None:
    for cell in ["P2", "N", "N0", "N43"]:
        assert len(rh.cell_to_boundary_densified(cell, points_per_edge=3)) == 8
        assert len(rh.cell_to_boundary_densified(cell, points_per_edge=5)) == 16

    batch = rh.cells_to_boundaries(
        ["Q4", "Q5", "N0", "N43"], points_per_edge=5, parallel=False
    )
    assert len(batch) == 4
    assert all(len(boundary) == 16 for boundary in batch)
    for cell, boundary in zip(["Q4", "Q5", "N0", "N43"], batch):
        scalar = rh.cell_to_boundary_densified(cell, points_per_edge=5)
        for actual, expected_point in zip(boundary, scalar):
            assert actual == pytest.approx(expected_point, abs=2e-10)

    expected = [
        (74.424006701996, 90.0),
        (58.52801748206219, 112.5),
        (41.93785391016014, 120.0),
        (41.93785391016014, 105.0),
        (41.93785391016014, 90.0),
        (41.93785391016014, 75.0),
        (41.93785391016014, 60.0),
        (58.52801748206219, 67.5),
    ]
    actual = rh.cell_to_boundary_densified("N0", points_per_edge=3)
    for point, reference in zip(actual, expected):
        assert point == pytest.approx(reference, abs=2e-10)


def test_densified_boundary_validation_and_interior_inset() -> None:
    with pytest.raises(ValueError, match="at least 2"):
        rh.cell_to_boundary_densified("P2", points_per_edge=1)
    outer = rh.cell_to_boundary_densified("N62", points_per_edge=3)
    inner = rh.cell_to_boundary_densified(
        "N62", points_per_edge=3, interior=True
    )
    assert inner != outer


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


def test_upstream_object_facade_boundary_semantics() -> None:
    dggs = rh.RHEALPixDGGS()
    assert len(dggs.cell("N0").boundary(n=3)) == 8
    assert len(dggs.cell("N0").boundary(n=3, plane=False)) == 8
    assert len(dggs.cell("N43").boundary(n=3, plane=False)) == 8
    assert len(dggs.cell("P2").boundary(n=3, plane=False)) == 8
    assert len(dggs.cell("N").boundary(n=3, plane=False)) == 8
    assert dggs.cell("P2").boundary(n=3, plane=False, interior=True) != (
        dggs.cell("P2").boundary(n=3, plane=False)
    )

    with pytest.raises(ValueError, match="at least 2"):
        dggs.cell("N0").boundary(n=1)


def test_upstream_object_facade_ordering_and_traversal() -> None:
    dggs = rh.RHEALPixDGGS()
    cells = [
        dggs.cell(value)
        for value in ["N", "N0", "N00", "N01", "N08", "N1", "O0"]
    ]
    assert [str(cell) for cell in sorted(cells)] == [
        "N00",
        "N01",
        "N08",
        "N0",
        "N1",
        "N",
        "O0",
    ]

    cell = dggs.cell("Q381")
    assert cell.index() == 3_049
    assert cell.index("post") == 795_604_004_266_974
    assert dggs.cell(level_order_index=cell.index()) == cell
    assert dggs.cell(post_order_index=cell.index("post")) == cell
    assert str(dggs.cell("N82").successor(3)) == "N830"
    assert str(dggs.cell("N08").predecessor(3)) == "N088"

    with pytest.raises(ValueError, match="exactly one"):
        dggs.cell()
    with pytest.raises(ValueError, match="exactly one"):
        dggs.cell("N", level_order_index=0)
    with pytest.raises(ValueError, match="order"):
        cell.index("unknown")


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
