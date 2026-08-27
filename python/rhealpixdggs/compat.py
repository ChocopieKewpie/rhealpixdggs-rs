"""Small upstream-compatible object facade backed by the Rust extension.

The functional API remains the preferred surface for new applications. These
classes ease migration from ``rhealpixdggs-py`` while the remaining geometry
operations are ported.
"""

from __future__ import annotations

from collections.abc import Iterator, Sequence
from itertools import product
from typing import Final

from ._rhealpixdggs import (
    MAX_RESOLUTION,
    _cell_from_point,
    _cell_metric,
    _cell_neighbor,
    _cell_neighbors,
    _cell_nucleus,
    _cell_vertices,
    get_cell_shape,
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

    def cell(self, suid: str | Sequence[str | int]) -> Cell:
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
