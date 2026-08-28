"""NumPy-native bulk operations backed by contiguous Rust buffers.

Inputs are normalized to little-endian, C-contiguous arrays and cross the
extension boundary once per batch. Returned arrays are zero-copy, read-only
views over immutable Python byte buffers.
"""

from __future__ import annotations

from typing import Optional, Tuple

import numpy as np
from numpy.typing import ArrayLike, NDArray

from ._rhealpixdggs import (
    BOUNDARY_PARALLEL_THRESHOLD,
    PARALLELISM_AVAILABLE,
    POINT_PARALLEL_THRESHOLD,
    REGION_PARALLEL_THRESHOLD,
    _bboxes_to_cells_buffer,
    _cells_to_boundaries_buffer,
    _cells_to_latlngs_buffer,
    _latlngs_to_cells_buffer,
)


def _matrix(values: ArrayLike, columns: int, name: str) -> NDArray[np.float64]:
    result = np.asarray(values, dtype="<f8", order="C")
    if result.ndim != 2 or result.shape[1] != columns:
        raise ValueError(f"{name} must have shape (n, {columns}); got {result.shape}")
    return np.ascontiguousarray(result, dtype="<f8")


def _cell_vector(values: ArrayLike) -> NDArray[np.uint64]:
    source = np.asarray(values, order="C")
    if source.ndim != 1:
        raise ValueError(f"cells must have shape (n,); got {source.shape}")
    if source.dtype.kind not in "iu":
        raise ValueError("cells must contain integer IDs")
    if source.dtype.kind == "i" and np.any(source < 0):
        raise ValueError("cell IDs cannot be negative")
    return np.ascontiguousarray(source, dtype="<u8")


def latlngs_to_cells(
    coordinates: ArrayLike,
    resolution: int,
    *,
    parallel: Optional[bool] = None,
) -> NDArray[np.uint64]:
    """Convert an ``(n, 2)`` latitude/longitude array to integer cell IDs.

    ``parallel=None`` selects Rayon at the measured point crossover;
    ``True`` forces it and ``False`` keeps execution on the calling thread.
    """

    values = _matrix(coordinates, 2, "coordinates")
    data = _latlngs_to_cells_buffer(values.tobytes(order="C"), resolution, parallel)
    return np.frombuffer(data, dtype="<u8")


def cells_to_latlngs(
    cells: ArrayLike, *, parallel: Optional[bool] = None
) -> NDArray[np.float64]:
    """Convert integer cell IDs to an ``(n, 2)`` nucleus array."""

    values = _cell_vector(cells)
    data = _cells_to_latlngs_buffer(values.tobytes(order="C"), parallel)
    return np.frombuffer(data, dtype="<f8").reshape(values.size, 2)


def cells_to_boundaries(
    cells: ArrayLike,
    *,
    points_per_edge: int = 2,
    interior: bool = False,
    parallel: Optional[bool] = None,
) -> NDArray[np.float64]:
    """Return fixed boundaries with shape ``(n, 4*p-4, 2)``.

    The last dimension is ``(latitude, longitude)``.
    """

    values = _cell_vector(cells)
    data = _cells_to_boundaries_buffer(
        values.tobytes(order="C"), points_per_edge, interior, parallel
    )
    point_count = 4 * points_per_edge - 4
    return np.frombuffer(data, dtype="<f8").reshape(values.size, point_count, 2)


def bboxes_to_cells(
    bboxes: ArrayLike,
    resolution: int,
    *,
    parallel: Optional[bool] = None,
) -> Tuple[NDArray[np.uint64], NDArray[np.uint64]]:
    """Cover many boxes and return ``(cells, offsets)`` ragged arrays.

    Each input row is ``(north, south, east, west)``. Cells for box ``i`` are
    ``cells[offsets[i]:offsets[i + 1]]``. A row with ``west > east`` crosses
    the antimeridian.
    """

    values = _matrix(bboxes, 4, "bboxes")
    cell_data, offset_data = _bboxes_to_cells_buffer(
        values.tobytes(order="C"), resolution, parallel
    )
    return (
        np.frombuffer(cell_data, dtype="<u8"),
        np.frombuffer(offset_data, dtype="<u8"),
    )


__all__ = [
    "BOUNDARY_PARALLEL_THRESHOLD",
    "PARALLELISM_AVAILABLE",
    "POINT_PARALLEL_THRESHOLD",
    "REGION_PARALLEL_THRESHOLD",
    "bboxes_to_cells",
    "cells_to_boundaries",
    "cells_to_latlngs",
    "latlngs_to_cells",
]
