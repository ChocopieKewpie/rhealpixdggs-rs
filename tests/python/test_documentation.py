"""Executable invariants used by the user documentation examples."""

from __future__ import annotations

import numpy as np

import rhealpixdggs as rh
from rhealpixdggs import numpy as rhnp


def test_python_quickstart_values() -> None:
    cell = rh.latlng_to_cell(-40.356, 175.611, 8)
    assert cell == "R88756047"
    assert rh.cell_to_parent(cell) == "R8875604"
    assert len(rh.cell_to_children(rh.cell_to_parent(cell))) == 9
    assert len(rh.cell_to_boundary_densified(cell, points_per_edge=8)) == 28
    assert rh.int_to_str(rh.str_to_int(cell)) == cell


def test_identifier_documentation_value() -> None:
    assert rh.str_to_int("Q381") == 3049
    assert rh.int_to_str(3049) == "Q381"


def test_numpy_quickstart_shapes() -> None:
    coordinates = np.array(
        [
            [-40.356, 175.611],
            [-41.2865, 174.7762],
            [40.7128, -74.0060],
        ]
    )
    cells = rhnp.latlngs_to_cells(coordinates, 8)
    boundaries = rhnp.cells_to_boundaries(cells, points_per_edge=8)
    assert cells.dtype == np.dtype("uint64")
    assert boundaries.shape == (3, 28, 2)

