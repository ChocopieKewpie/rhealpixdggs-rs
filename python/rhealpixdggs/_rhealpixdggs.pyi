from collections.abc import Sequence

MAX_RESOLUTION: int
__version__: str

def latlng_to_cell(latitude: float, longitude: float, resolution: int) -> str: ...
def latlngs_to_cells(
    coordinates: Sequence[tuple[float, float]], resolution: int
) -> list[str]: ...
def cell_to_latlng(cell: str) -> tuple[float, float]: ...
def cell_to_centroid(cell: str) -> tuple[float, float]: ...
def bbox_to_cells(
    north: float, south: float, east: float, west: float, resolution: int
) -> list[str]: ...
def line_to_cells(
    coordinates: Sequence[tuple[float, float]], resolution: int
) -> list[str]: ...
def polygon_to_cells(
    exterior: Sequence[tuple[float, float]],
    resolution: int,
    holes: Sequence[Sequence[tuple[float, float]]] | None = None,
    compact: bool = False,
) -> list[str]: ...
def cell_to_boundary(
    cell: str, trim_dart: bool = False
) -> list[tuple[float, float]]: ...
def cell_to_boundary_densified(
    cell: str, points_per_edge: int = 2, interior: bool = False
) -> list[tuple[float, float]]: ...
def get_cell_region(cell: str) -> str: ...
def get_cell_shape(cell: str) -> str: ...
def cell_to_neighbor(
    cell: str, direction: str, plane: bool = True
) -> str | None: ...
def cell_to_neighbors(cell: str, plane: bool = True) -> dict[str, str]: ...
def cell_to_parent(cell: str, resolution: int | None = None) -> str | None: ...
def cell_to_children(cell: str, resolution: int | None = None) -> list[str]: ...
def cell_to_successor(cell: str, resolution: int | None = None) -> str | None: ...
def cell_to_predecessor(cell: str, resolution: int | None = None) -> str | None: ...
def cell_to_level_order_index(cell: str) -> int: ...
def level_order_index_to_cell(index: int) -> str: ...
def cell_to_post_order_index(cell: str) -> int: ...
def post_order_index_to_cell(index: int) -> str: ...
def get_resolution(cell: str) -> int: ...
def get_base_cell_number(cell: str) -> int: ...
def is_valid_cell(cell: str) -> bool: ...
def str_to_int(cell: str) -> int: ...
def int_to_str(cell: int) -> str: ...
def cell_area(cell: str, unit: str = "m^2") -> float: ...
def compact_cells(cells: Sequence[str]) -> list[str]: ...
def uncompact_cells(cells: Sequence[str], resolution: int) -> list[str]: ...

def _cell_from_point(
    resolution: int,
    point: tuple[float, float],
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> str | None: ...
def _project(
    point: tuple[float, float],
    projection: str = "rhealpix",
    inverse: bool = False,
    region: str = "none",
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[float, float]: ...
def _combine_triangles(
    point: tuple[float, float],
    inverse: bool = False,
    region: str = "none",
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[float, float]: ...
def _triangle(
    point: tuple[float, float],
    inverse: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[int | None, str]: ...
def _xyz(
    point: tuple[float, float],
    lonlat: bool = False,
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[float, float, float]: ...
def _xyz_cube(
    point: tuple[float, float],
    lonlat: bool = False,
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[float, float, float]: ...
def _cell_nucleus(
    cell: str,
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[float, float]: ...
def _cell_centroid(
    cell: str,
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[float, float]: ...
def _cells_from_region(
    resolution: int,
    upper_left: tuple[float, float],
    lower_right: tuple[float, float],
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> list[list[str]]: ...
def _cells_from_line(
    resolution: int,
    start: tuple[float, float],
    end: tuple[float, float],
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> list[str]: ...
def _cell_vertices(
    cell: str,
    plane: bool = True,
    trim_dart: bool = False,
    north_square: int = 0,
    south_square: int = 0,
) -> list[tuple[float, float]]: ...
def _cell_vertex(
    cell: str,
    vertex: str = "upper_left",
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> tuple[float, float]: ...
def _cell_boundary(
    cell: str,
    n: int = 2,
    plane: bool = True,
    interior: bool = False,
    north_square: int = 0,
    south_square: int = 0,
) -> list[tuple[float, float]]: ...
def _cell_neighbor(
    cell: str,
    direction: str,
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> str | None: ...
def _cell_neighbors(
    cell: str,
    plane: bool = True,
    north_square: int = 0,
    south_square: int = 0,
) -> list[tuple[str, str]]: ...
def _cell_metric(
    resolution: int, metric: str, plane: bool = True
) -> float | None: ...
def _compare_cells(left: str, right: str) -> int: ...
