"""Fast rHEALPix DGGS indexing powered by Rust.

Coordinates follow H3's Python convention: public functions accept and return
``(latitude, longitude)`` while the Rust core uses conventional GIS
``(longitude, latitude)`` ordering.
"""

from ._rhealpixdggs import (
    MAX_RESOLUTION,
    __version__,
    cell_area,
    cell_to_boundary,
    cell_to_children,
    cell_to_latlng,
    cell_to_parent,
    compact_cells,
    get_base_cell_number,
    get_resolution,
    int_to_str,
    is_valid_cell,
    latlng_to_cell,
    latlngs_to_cells,
    str_to_int,
    uncompact_cells,
)

__all__ = [
    "MAX_RESOLUTION",
    "__version__",
    "cell_area",
    "cell_to_boundary",
    "cell_to_children",
    "cell_to_latlng",
    "cell_to_parent",
    "compact_cells",
    "get_base_cell_number",
    "get_resolution",
    "int_to_str",
    "is_valid_cell",
    "latlng_to_cell",
    "latlngs_to_cells",
    "str_to_int",
    "uncompact_cells",
]

