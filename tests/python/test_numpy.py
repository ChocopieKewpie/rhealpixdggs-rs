import threading

import numpy as np
import pytest

import rhealpixdggs as rh
from rhealpixdggs import numpy as rhnp


def test_numpy_point_round_trip_matches_scalar_api() -> None:
    points = np.array(
        [[-41.2865, 174.7762], [0.0, 0.0], [10.0, 179.0], [72.5, -179.999]],
        dtype=np.float64,
    )
    cells = rhnp.latlngs_to_cells(points, 7, parallel=False)
    expected = np.array(
        [rh.str_to_int(rh.latlng_to_cell(lat, lng, 7)) for lat, lng in points],
        dtype=np.uint64,
    )
    np.testing.assert_array_equal(cells, expected)
    assert cells.dtype == np.dtype("<u8")
    assert not cells.flags.writeable

    nuclei = rhnp.cells_to_latlngs(cells, parallel=True)
    assert nuclei.shape == points.shape
    for nucleus, cell in zip(nuclei, cells):
        assert rh.str_to_int(rh.latlng_to_cell(*nucleus, 7)) == cell


def test_numpy_boundaries_match_scalar_api_for_every_shape() -> None:
    identifiers = ["P2", "N", "N0", "N43"]
    cells = np.array([rh.str_to_int(cell) for cell in identifiers], dtype=np.uint64)
    boundaries = rhnp.cells_to_boundaries(
        cells, points_per_edge=3, interior=True, parallel=True
    )
    assert boundaries.shape == (4, 8, 2)
    for identifier, boundary in zip(identifiers, boundaries):
        np.testing.assert_allclose(
            boundary,
            rh.cell_to_boundary_densified(
                identifier, points_per_edge=3, interior=True
            ),
            rtol=0.0,
            atol=2e-10,
        )


def test_numpy_bbox_cover_uses_csr_offsets_and_splits_antimeridian() -> None:
    boxes = np.array(
        [[-40.0, -42.0, 176.0, 173.0], [11.0, 9.0, -178.0, 178.0]]
    )
    cells, offsets = rhnp.bboxes_to_cells(boxes, 3, parallel=True)
    np.testing.assert_array_equal(offsets[[0, -1]], [0, len(cells)])
    for index, (north, south, east, west) in enumerate(boxes):
        actual = cells[offsets[index] : offsets[index + 1]]
        expected = np.array(
            [
                rh.str_to_int(cell)
                for cell in rh.bbox_to_cells(north, south, east, west, 3)
            ],
            dtype=np.uint64,
        )
        np.testing.assert_array_equal(actual, expected)


@pytest.mark.parametrize(
    ("call", "message"),
    [
        (lambda: rhnp.latlngs_to_cells([1.0, 2.0], 3), "shape"),
        (lambda: rhnp.cells_to_latlngs([[1]], parallel=False), "shape"),
        (lambda: rhnp.cells_to_latlngs([1.5]), "integer IDs"),
        (lambda: rhnp.cells_to_latlngs([-1]), "negative"),
        (lambda: rhnp.bboxes_to_cells([[1.0, 2.0]], 3), "shape"),
        (
            lambda: rhnp.cells_to_boundaries(
                np.array([rh.str_to_int("P2")]), points_per_edge=1
            ),
            "at least 2",
        ),
    ],
)
def test_numpy_validation(call, message: str) -> None:
    with pytest.raises(ValueError, match=message):
        call()


def test_empty_numpy_batches_keep_shapes_and_validate_resolution() -> None:
    cells = rhnp.latlngs_to_cells(np.empty((0, 2)), 3)
    assert cells.shape == (0,)
    assert rhnp.cells_to_latlngs(cells).shape == (0, 2)
    assert rhnp.cells_to_boundaries(cells).shape == (0, 4, 2)
    covered, offsets = rhnp.bboxes_to_cells(np.empty((0, 4)), 3)
    assert covered.shape == (0,)
    np.testing.assert_array_equal(offsets, [0])
    with pytest.raises(ValueError, match="resolution"):
        rhnp.latlngs_to_cells(np.empty((0, 2)), 16)


def test_bulk_point_work_releases_the_gil() -> None:
    # A second Python thread must make progress while sequential Rust work is
    # active. This is intentionally a coarse contract test, not a benchmark.
    points = np.column_stack(
        (
            np.linspace(-89.0, 89.0, 2_000_000),
            np.linspace(-179.0, 179.0, 2_000_000),
        )
    )
    started = threading.Event()
    finished = threading.Event()

    def convert() -> None:
        started.set()
        rhnp.latlngs_to_cells(points, 9, parallel=False)
        finished.set()

    thread = threading.Thread(target=convert)
    thread.start()
    started.wait()
    progress = 0
    while not finished.is_set():
        progress += 1
    thread.join()
    assert progress > 0
