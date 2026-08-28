"""Small upstream-compatible object facade backed by the Rust extension.

The functional API remains the preferred surface for new applications. These
classes ease migration from ``rhealpixdggs-py`` while the remaining geometry
operations are ported.
"""

from __future__ import annotations

from collections.abc import Iterator, Sequence
from functools import total_ordering
from itertools import product
from math import ceil, pi
import sys
from typing import Final

from ._rhealpixdggs import (
    MAX_RESOLUTION,
    _combine_triangles,
    _compare_cells,
    _cell_boundary,
    _cell_centroid,
    _cell_from_point,
    _cell_metric,
    _cell_neighbor,
    _cell_neighbors,
    _cell_nucleus,
    _cell_vertices,
    _cell_vertex,
    _cells_from_line,
    _cells_from_region,
    _project,
    _triangle,
    _xyz,
    _xyz_cube,
    cell_to_level_order_index,
    cell_to_post_order_index,
    cell_to_predecessor,
    cell_to_successor,
    get_cell_shape,
    level_order_index_to_cell,
    post_order_index_to_cell,
)

_FACES: Final = frozenset("NOPQRS")
_PLANAR_DIRECTIONS: Final = ("left", "right", "down", "up")


def _identifier(suid: str | Sequence[str | int]) -> str:
    if isinstance(suid, str):
        identifier = suid
    else:
        if not suid:
            raise ValueError("cell SUID cannot be empty")
        identifier = str(suid[0]) + "".join(str(value) for value in suid[1:])
    if not identifier or identifier[0] not in _FACES:
        raise ValueError(f"invalid cell SUID: {suid!r}")
    if len(identifier) > MAX_RESOLUTION + 1 or any(
        digit not in "012345678" for digit in identifier[1:]
    ):
        raise ValueError(f"invalid cell SUID: {suid!r}")
    return identifier


class RHEALPixDGGS:
    """Aperture-9 WGS84 rHEALPix DGGS compatibility facade.

    ``north_square`` and ``south_square`` are fully supported. Alternate
    ellipsoids and ``N_side`` values other than 3 are not yet implemented.
    """

    N_side: Final = 3
    max_resolution: Final = MAX_RESOLUTION

    def __init__(
        self,
        ellipsoid: object | None = None,
        north_square: int = 0,
        south_square: int = 0,
        N_side: int = 3,
        **_: object,
    ) -> None:
        if ellipsoid is not None:
            raise NotImplementedError("custom ellipsoids are not implemented yet")
        if N_side != 3:
            raise NotImplementedError("only aperture-9 (N_side=3) is supported")
        self.north_square = int(north_square) % 4
        self.south_square = int(south_square) % 4

    def __repr__(self) -> str:
        return (
            "RHEALPixDGGS("
            f"north_square={self.north_square}, south_square={self.south_square})"
        )

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, RHEALPixDGGS)
            and self.north_square == other.north_square
            and self.south_square == other.south_square
        )

    def healpix(
        self, u: float, v: float, inverse: bool = False
    ) -> tuple[float, float]:
        """Apply the WGS84 HEALPix projection or its inverse."""
        return _project((u, v), "healpix", inverse)

    def rhealpix(
        self,
        u: float,
        v: float,
        inverse: bool = False,
        region: str = "none",
    ) -> tuple[float, float]:
        """Apply this grid's rHEALPix projection or its inverse."""
        return _project(
            (u, v),
            "rhealpix",
            inverse,
            region,
            self.north_square,
            self.south_square,
        )

    def combine_triangles(
        self,
        u: float,
        v: float,
        inverse: bool = False,
        region: str = "none",
    ) -> tuple[float, float]:
        """Transform between the HEALPix and rHEALPix projected images."""
        return _combine_triangles(
            (u, v),
            inverse,
            region,
            self.north_square,
            self.south_square,
        )

    def triangle(
        self, x: float, y: float, inverse: bool = True
    ) -> tuple[int | None, str]:
        """Return the HEALPix polar triangle and geographic region."""
        return _triangle(
            (x, y), inverse, self.north_square, self.south_square
        )

    def xyz(
        self, u: float, v: float, lonlat: bool = False
    ) -> tuple[float, float, float]:
        """Return geocentric Cartesian coordinates on WGS84."""
        return _xyz(
            (u, v), lonlat, self.north_square, self.south_square
        )

    def xyz_cube(
        self, u: float, v: float, lonlat: bool = False
    ) -> tuple[float, float, float]:
        """Fold the rHEALPix image onto a cube centred at the origin."""
        return _xyz_cube(
            (u, v), lonlat, self.north_square, self.south_square
        )

    def cell(
        self,
        suid: str | Sequence[str | int] | None = None,
        level_order_index: int | None = None,
        post_order_index: int | None = None,
    ) -> Cell:
        """Construct a cell from exactly one identifier or traversal index."""
        supplied = sum(
            value is not None
            for value in (suid, level_order_index, post_order_index)
        )
        if supplied != 1:
            raise ValueError(
                "provide exactly one of suid, level_order_index, or post_order_index"
            )
        if level_order_index is not None:
            suid = level_order_index_to_cell(level_order_index)
        elif post_order_index is not None:
            suid = post_order_index_to_cell(post_order_index)
        assert suid is not None
        return Cell(self, suid)

    def cell_from_point(
        self, resolution: int, p: tuple[float, float], plane: bool = True
    ) -> Cell | None:
        identifier = _cell_from_point(
            resolution,
            p,
            plane,
            self.north_square,
            self.south_square,
        )
        return None if identifier is None else Cell(self, identifier)

    def interval(self, a: Cell, b: Cell) -> Iterator[Cell]:
        """Yield the fixed-resolution post-order interval ``[a, b]``."""
        resolution = max(a.resolution, b.resolution)
        cell = (
            a.successor(resolution)
            if a.resolution < resolution
            else Cell(self, a.suid[: resolution + 1])
        )
        while cell is not None and cell <= b:
            yield cell
            cell = cell.successor(resolution)

    def cell_from_region(
        self,
        upper_left: tuple[float, float],
        lower_right: tuple[float, float],
        plane: bool = True,
    ) -> Cell | None:
        """Return the smallest cell wholly containing an axis-aligned region."""
        if not plane:
            if upper_left == (-180.0, 90.0) or lower_right == (-180.0, -90.0):
                latitude = (
                    lower_right[1]
                    if lower_right[1] != -90.0
                    else upper_left[1]
                )
                vertices = [
                    (-135.0, latitude),
                    (-45.0, latitude),
                    (45.0, latitude),
                    (135.0, latitude),
                ]
            else:
                vertices = [
                    upper_left,
                    (upper_left[0], lower_right[1]),
                    lower_right,
                    (lower_right[0], upper_left[1]),
                ]
            projected = [self.rhealpix(*point) for point in vertices]
            upper_left = (
                min(point[0] for point in projected),
                max(point[1] for point in projected),
            )
            lower_right = (
                max(point[0] for point in projected),
                min(point[1] for point in projected),
            )

        upper = self.cell_from_point(MAX_RESOLUTION, upper_left)
        lower = self.cell_from_point(MAX_RESOLUTION, lower_right)
        if upper is None or lower is None:
            return None
        common = 0
        for left, right in zip(upper.suid, lower.suid):
            if left != right:
                break
            common += 1
        return None if common == 0 else Cell(self, upper.suid[:common])

    def cell_latitudes(
        self,
        resolution: int,
        phi_min: float,
        phi_max: float,
        nucleus: bool = True,
        plane: bool = True,
    ) -> list[float]:
        """Return cell nucleus or boundary latitudes inside an open interval."""
        if phi_min > phi_max:
            return []
        root_width = self.cell_width(0)
        assert root_width is not None
        radius = 2.0 * root_width / pi
        if plane:
            y_min, y_max = phi_min, phi_max
        else:
            y_min = self.healpix(0.0, phi_min)[1]
            y_max = self.healpix(0.0, phi_max)[1]
        width = self.cell_width(resolution)
        assert width is not None
        y = -radius * pi / 2.0 + (width if nucleus else width / 2.0)
        if y <= y_min:
            difference = y_min - y
            y = max(y + ceil(difference / width) * width, y + width)
        result: list[float] = []
        while y < y_max:
            result.append(y)
            y += width
        if plane:
            return result
        return [self.healpix(radius * pi / 4.0, y, inverse=True)[1] for y in result]

    def cells_from_meridian(
        self, resolution: int, lam: float, phi_min: float, phi_max: float
    ) -> list[Cell]:
        """Return cells intersecting a geographic meridian segment."""
        if phi_min > phi_max:
            return []
        start = self.cell_from_point(resolution, (lam, phi_max), plane=False)
        end = self.cell_from_point(resolution, (lam, phi_min), plane=False)
        if start is None or end is None:
            return []
        if start == end:
            return [start]
        latitudes = self.cell_latitudes(
            resolution, phi_min, phi_max, nucleus=True, plane=False
        )
        if not latitudes:
            return [start, end]
        result: list[Cell] = []
        for latitude in reversed(latitudes):
            cell = self.cell_from_point(resolution, (lam, latitude), plane=False)
            assert cell is not None
            new_cells = [cell]
            if cell.ellipsoidal_shape in {"dart", "skew_quad"}:
                west = cell.neighbor("west", plane=False)
                east = cell.neighbor("east", plane=False)
                if west is not None and west.intersects_meridian(lam):
                    new_cells = [west, cell]
                elif east is not None and east.intersects_meridian(lam):
                    new_cells = [cell, east]
            result.extend(new_cells)
        if start not in result[:2]:
            result.insert(0, start)
        if end not in result[-2:]:
            result.append(end)
        return result

    def cells_from_parallel(
        self, resolution: int, phi: float, lam_min: float, lam_max: float
    ) -> list[Cell]:
        """Return cells intersecting a geographic parallel segment."""
        if lam_min > lam_max:
            return []
        start = self.cell_from_point(resolution, (lam_min, phi), plane=False)
        end = self.cell_from_point(resolution, (lam_max, phi), plane=False)
        if start is None or end is None:
            return []
        if start == end:
            if start.ellipsoidal_shape == "cap" or lam_max - lam_min < 90.0:
                return [start]
            neighbor = start.neighbor("west", plane=False)
            if neighbor is None:
                return [start]
            end = neighbor
        result: list[Cell] = []
        current = start
        while current != end:
            result.append(current)
            neighbor = current.neighbor("east", plane=False)
            if neighbor is None:
                raise RuntimeError("parallel traversal reached a cell without an east neighbor")
            current = neighbor
        result.append(end)
        return result

    def cells_from_region(
        self,
        resolution: int,
        upper_left: tuple[float, float],
        lower_right: tuple[float, float],
        plane: bool = True,
    ) -> list[list[Cell]]:
        """Return cells covering an axis-aligned region in upstream order."""
        return [
            [Cell(self, identifier) for identifier in row]
            for row in _cells_from_region(
                resolution,
                upper_left,
                lower_right,
                plane,
                self.north_square,
                self.south_square,
            )
        ]

    def cells_from_line(
        self,
        resolution: int,
        start: tuple[float, float],
        end: tuple[float, float],
        plane: bool = True,
    ) -> list[Cell]:
        """Return cells touched by a two-point line in path order."""
        return [
            Cell(self, identifier)
            for identifier in _cells_from_line(
                resolution,
                start,
                end,
                plane,
                self.north_square,
                self.south_square,
            )
        ]

    def minimal_cover(
        self,
        resolution: int,
        points: Sequence[tuple[float, float]],
        plane: bool = True,
    ) -> list[Cell]:
        """Return the distinct resolution cells containing ``points``."""
        cover: dict[str, Cell] = {}
        for point in points:
            cell = self.cell_from_point(resolution, point, plane)
            if cell is not None:
                cover[str(cell)] = cell
        return list(cover.values())

    @staticmethod
    def antimeridian_check_and_flip(
        vertices: list[tuple[float, float]], plane: bool = True
    ) -> list[tuple[float, float]]:
        """Normalize antimeridian vertices to the side used by their peers."""
        if plane:
            return vertices
        longitudes = [point[0] for point in vertices]
        if 180.0 not in longitudes and -180.0 not in longitudes:
            return vertices
        check = 180.0 if 180.0 in longitudes else -180.0
        if all(value == check or value * check >= 0.0 for value in longitudes):
            return vertices
        return [
            (-longitude if longitude == check else longitude, latitude)
            for longitude, latitude in vertices
        ]

    def grid(self, resolution: int) -> Iterator[Cell]:
        if not 0 <= resolution <= MAX_RESOLUTION:
            raise ValueError(f"resolution must be in [0, {MAX_RESOLUTION}]")
        for face in "NOPQRS":
            yield from Cell(self, face).subcells(resolution)

    def num_cells(
        self, res_1: int, res_2: int | None = None, subcells: bool = False
    ) -> int:
        if res_2 is None or res_2 < res_1:
            res_2 = MAX_RESOLUTION if subcells else res_1
        if subcells:
            return (9 ** (res_2 - res_1 + 1) - 1) // 8
        return 6 * (9 ** (res_2 + 1) - 9**res_1) // 8

    def cell_width(self, resolution: int, plane: bool = True) -> float | None:
        return _cell_metric(resolution, "width", plane)

    def cell_area(self, resolution: int, plane: bool = True) -> float:
        value = _cell_metric(resolution, "area", plane)
        assert value is not None
        return value

    def area_error_budget(self) -> dict[int, dict[str, float]]:
        """Return conservative floating-point tolerances for cell areas."""
        relative = 10.0 * sys.float_info.epsilon
        return {
            resolution: {
                "cell_area_m2": area,
                "abs_tolerance": area * relative,
                "rel_tolerance": relative,
            }
            for resolution in range(MAX_RESOLUTION + 1)
            if (area := self.cell_area(resolution, plane=False)) is not None
        }


@total_ordering
class Cell:
    """Upstream-style cell object retaining its parent DGGS configuration."""

    def __init__(
        self, rdggs: RHEALPixDGGS, suid: str | Sequence[str | int]
    ) -> None:
        if not isinstance(rdggs, RHEALPixDGGS):
            raise TypeError("rdggs must be a RHEALPixDGGS instance")
        self.rdggs = rdggs
        self._identifier = _identifier(suid)
        self.suid = (self._identifier[0],) + tuple(
            int(digit) for digit in self._identifier[1:]
        )
        self.resolution = len(self._identifier) - 1

    def __str__(self) -> str:
        return self._identifier

    def __repr__(self) -> str:
        return f"Cell({self.rdggs!r}, {self.suid!r})"

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, Cell)
            and self.rdggs == other.rdggs
            and self.suid == other.suid
        )

    def __lt__(self, other: object) -> bool:
        if not isinstance(other, Cell):
            return NotImplemented
        return _compare_cells(self._identifier, other._identifier) < 0

    @property
    def north_square(self) -> int:
        return self.rdggs.north_square

    @property
    def south_square(self) -> int:
        return self.rdggs.south_square

    @property
    def N_side(self) -> int:
        return self.rdggs.N_side

    @property
    def ellipsoidal_shape(self) -> str:
        return get_cell_shape(self._identifier)

    def region(self) -> str:
        return {
            "N": "north_polar",
            "S": "south_polar",
        }.get(self._identifier[0], "equatorial")

    def nucleus(self, plane: bool = True) -> tuple[float, float]:
        return _cell_nucleus(
            self._identifier,
            plane,
            self.north_square,
            self.south_square,
        )

    def centroid(self, plane: bool = True) -> tuple[float, float]:
        """Return the planar or ellipsoidal centroid of this cell."""
        return _cell_centroid(
            self._identifier,
            plane,
            self.north_square,
            self.south_square,
        )

    def vertices(
        self, plane: bool = True, trim_dart: bool = False
    ) -> list[tuple[float, float]]:
        return _cell_vertices(
            self._identifier,
            plane,
            trim_dart,
            self.north_square,
            self.south_square,
        )

    def boundary(
        self, n: int = 2, plane: bool = True, interior: bool = False
    ) -> list[tuple[float, float]]:
        """Return the upstream-compatible clockwise cell boundary.

        Planar boundaries contain exactly ``4*n - 4`` points. Geographic
        quad and cap cells retain upstream's four-vertex shortcut; dart and
        skew-quad cells contain ``4*n - 4`` points.
        """
        return _cell_boundary(
            self._identifier,
            n,
            plane,
            interior,
            self.north_square,
            self.south_square,
        )

    def suid_rowcol(
        self,
    ) -> tuple[tuple[str | int, ...], tuple[str | int, ...]]:
        """Return the row and column SUID components."""
        rows: list[str | int] = [self.suid[0]]
        columns: list[str | int] = [self.suid[0]]
        for digit in self.suid[1:]:
            assert isinstance(digit, int)
            rows.append(digit // 3)
            columns.append(digit % 3)
        return tuple(rows), tuple(columns)

    def subcell(self, other: Cell) -> bool:
        """Return whether this cell is a descendant of ``other``."""
        return self._identifier.startswith(other._identifier)

    @staticmethod
    def rotate_entry(x: str | int, quarter_turns: int) -> str | int:
        """Rotate one aperture-9 digit anticlockwise."""
        if isinstance(x, str):
            return x
        row, column = divmod(x, 3)
        for _ in range(quarter_turns % 4):
            row, column = column, 2 - row
        return row * 3 + column

    def rotate(self, quarter_turns: int) -> Cell:
        """Rotate every hierarchy digit anticlockwise."""
        return Cell(
            self.rdggs,
            tuple(self.rotate_entry(value, quarter_turns) for value in self.suid),
        )

    def ul_vertex(self, plane: bool = True) -> tuple[float, float]:
        return _cell_vertex(
            self._identifier,
            "upper_left",
            plane,
            self.north_square,
            self.south_square,
        )

    def nw_vertex(self, plane: bool = True) -> tuple[float, float]:
        return _cell_vertex(
            self._identifier,
            "northwest",
            plane,
            self.north_square,
            self.south_square,
        )

    def xy_range(self) -> tuple[tuple[float, float], tuple[float, float]]:
        upper_left = self.ul_vertex()
        width = self.width()
        assert width is not None
        return (
            (upper_left[0], upper_left[0] + width),
            (upper_left[1] - width, upper_left[1]),
        )

    def interior(
        self, n: int = 2, plane: bool = True, flatten: bool = False
    ) -> list[tuple[float, float]] | list[list[tuple[float, float]]]:
        """Return an upstream-compatible interior point grid."""
        if n < 2:
            raise ValueError("n must be at least 2")
        upper_left = self.ul_vertex()
        width = self.width()
        assert width is not None
        epsilon = 1e-6
        delta = (width - 2.0 * epsilon) / (n - 1)

        def point(row: int, column: int) -> tuple[float, float]:
            projected = (
                upper_left[0] + epsilon + delta * column,
                upper_left[1] - epsilon - delta * row,
            )
            return (
                projected
                if plane
                else self.rdggs.rhealpix(*projected, inverse=True)
            )

        if flatten:
            return [point(row, column) for column in range(n) for row in range(n)]
        return [[point(row, column) for column in range(n)] for row in range(n)]

    def contains(self, p: tuple[float, float], plane: bool = True) -> bool:
        return self.rdggs.cell_from_point(self.resolution, p, plane) == self

    def intersects_meridian(self, lam: float) -> bool:
        if self.ellipsoidal_shape == "cap":
            return True
        vertices = self.vertices(plane=False)
        longitude_min = min(point[0] for point in vertices)
        longitude_max = max(point[0] for point in vertices)
        if abs(longitude_min - longitude_max) > 180.0:
            longitude_min = -longitude_max
            return longitude_max <= lam or lam <= longitude_min
        return longitude_min <= lam <= longitude_max

    def intersects_parallel(self, phi: float) -> bool:
        vertices = self.vertices(plane=False)
        latitude_min = min(point[1] for point in vertices)
        latitude_max = max(point[1] for point in vertices)
        if self.ellipsoidal_shape == "cap":
            return (
                phi >= latitude_min
                if self.region() == "north_polar"
                else phi <= latitude_max
            )
        return latitude_min <= phi <= latitude_max

    def overlaps(self, other_cell: Cell) -> bool:
        length = min(len(self._identifier), len(other_cell._identifier))
        return self._identifier[:length] == other_cell._identifier[:length]

    def region_overlaps(self, region: Sequence[Cell]) -> bool:
        return any(self.overlaps(cell) for cell in region)

    def index(self, order: str = "resolution") -> int:
        """Return the level- or post-order index of this cell."""
        if order == "resolution":
            return cell_to_level_order_index(self._identifier)
        if order == "post":
            return cell_to_post_order_index(self._identifier)
        raise ValueError("order must be 'resolution' or 'post'")

    def successor(self, resolution: int | None = None) -> Cell | None:
        identifier = cell_to_successor(self._identifier, resolution)
        return None if identifier is None else Cell(self.rdggs, identifier)

    def predecessor(self, resolution: int | None = None) -> Cell | None:
        identifier = cell_to_predecessor(self._identifier, resolution)
        return None if identifier is None else Cell(self.rdggs, identifier)

    def neighbor(self, direction: str, plane: bool = True) -> Cell | None:
        identifier = _cell_neighbor(
            self._identifier,
            direction,
            plane,
            self.north_square,
            self.south_square,
        )
        return None if identifier is None else Cell(self.rdggs, identifier)

    def neighbors(self, plane: bool = True) -> dict[str, Cell]:
        if plane:
            return {
                direction: neighbor
                for direction in _PLANAR_DIRECTIONS
                if (neighbor := self.neighbor(direction, plane=True)) is not None
            }
        return {
            direction: Cell(self.rdggs, identifier)
            for direction, identifier in _cell_neighbors(
                self._identifier,
                False,
                self.north_square,
                self.south_square,
            )
        }

    def subcells(self, resolution: int | None = None) -> Iterator[Cell]:
        if resolution is None:
            resolution = self.resolution + 1
        if not 0 <= resolution <= MAX_RESOLUTION:
            raise ValueError(f"resolution must be in [0, {MAX_RESOLUTION}]")
        if resolution < self.resolution:
            return
        for suffix in product(range(9), repeat=resolution - self.resolution):
            yield Cell(self.rdggs, self.suid + suffix)

    def width(self, plane: bool = True) -> float | None:
        return self.rdggs.cell_width(self.resolution, plane)

    def area(self, plane: bool = True) -> float:
        return self.rdggs.cell_area(self.resolution, plane)


WGS84_003: Final = RHEALPixDGGS()
