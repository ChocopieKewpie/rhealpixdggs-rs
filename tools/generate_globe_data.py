#!/usr/bin/env python3
"""Generate the self-contained GeoJSON used by the documentation globe.

The land cover is calculated with the public Python API at resolution 5 and
then compacted. Natural Earth 1:110m land polygons are already bundled in the
repository for the README cover, so regeneration needs no network access and
no optional GIS packages.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
from pathlib import Path
from typing import Any, Optional, Sequence


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "docs" / "data" / "ne_110m_land.shp"
CELLS_OUTPUT = ROOT / "docs" / "data" / "rhealpix-land-r5-compacted.geojson"
UNCOMPACTED_OUTPUT = (
    ROOT / "docs" / "data" / "rhealpix-land-r5-uncompacted-grid.geojson"
)
COASTS_OUTPUT = ROOT / "docs" / "data" / "natural-earth-coastlines-110m.geojson"
Point = tuple[float, float]
Ring = list[Point]


def _load_rhealpix() -> Any:
    try:
        import rhealpixdggs as rh
    except ImportError as error:
        raise SystemExit(
            "rhealpixdggs is not importable. Activate the development conda "
            "environment and run `maturin develop --release` first."
        ) from error
    return rh


def _read_polygon_records(path: Path) -> list[list[Ring]]:
    """Read polygon records from an ESRI shapefile without a GIS dependency."""

    data = path.read_bytes()
    if len(data) < 100 or struct.unpack_from(">i", data, 0)[0] != 9994:
        raise ValueError(f"invalid shapefile header: {path}")

    records: list[list[Ring]] = []
    offset = 100
    while offset + 8 <= len(data):
        content_words = struct.unpack_from(">i", data, offset + 4)[0]
        content_start = offset + 8
        content_end = content_start + content_words * 2
        if content_end > len(data):
            raise ValueError(f"truncated shapefile record: {path}")

        shape_type = struct.unpack_from("<i", data, content_start)[0]
        if shape_type == 5:
            part_count, point_count = struct.unpack_from(
                "<ii", data, content_start + 36
            )
            part_start = content_start + 44
            point_start = part_start + 4 * part_count
            starts = list(
                struct.unpack_from(f"<{part_count}i", data, part_start)
            )
            starts.append(point_count)
            points = [
                struct.unpack_from("<dd", data, point_start + 16 * index)
                for index in range(point_count)
            ]
            rings = []
            for start, end in zip(starts, starts[1:]):
                ring = [
                    (
                        max(-180.0, min(180.0, float(longitude))),
                        max(-90.0, min(90.0, float(latitude))),
                    )
                    for longitude, latitude in points[start:end]
                ]
                ring = _open_ring(ring)
                if len(ring) >= 3:
                    rings.append(ring)
            if rings:
                records.append(rings)
        elif shape_type != 0:
            raise ValueError(f"unexpected shapefile shape type {shape_type}")
        offset = content_end
    return records


def _open_ring(points: Sequence[Point]) -> Ring:
    result: Ring = []
    for point in points:
        if not result or point != result[-1]:
            result.append(point)
    if len(result) > 1 and result[0] == result[-1]:
        result.pop()
    return result


def _signed_area(ring: Sequence[Point]) -> float:
    origin_x, origin_y = ring[0]
    area = 0.0
    for start, end in zip(ring, (*ring[1:], ring[0])):
        x1, y1 = start[0] - origin_x, start[1] - origin_y
        x2, y2 = end[0] - origin_x, end[1] - origin_y
        area += x1 * y2 - x2 * y1
    return area / 2.0


def _contains_point(ring: Sequence[Point], point: Point) -> bool:
    x, y = point
    inside = False
    previous = ring[-1]
    for current in ring:
        x1, y1 = previous
        x2, y2 = current
        if (y1 > y) != (y2 > y):
            crossing_x = (x2 - x1) * (y - y1) / (y2 - y1) + x1
            if x < crossing_x:
                inside = not inside
        previous = current
    return inside


def _polygon_parts(record: Sequence[Ring]) -> list[tuple[Ring, list[Ring]]]:
    """Associate counter-clockwise shapefile holes with clockwise exteriors."""

    exteriors = [ring for ring in record if _signed_area(ring) < 0.0]
    holes = [ring for ring in record if _signed_area(ring) > 0.0]
    if not exteriors:
        # Defensive fallback for sources whose winding convention is reversed.
        largest = max(record, key=lambda ring: abs(_signed_area(ring)))
        exteriors = [largest]
        holes = [ring for ring in record if ring is not largest]

    result = [(exterior, []) for exterior in exteriors]
    for hole in holes:
        candidates = [
            (abs(_signed_area(exterior)), index)
            for index, exterior in enumerate(exteriors)
            if _contains_point(exterior, hole[0])
        ]
        if candidates:
            _, index = min(candidates)
            result[index][1].append(hole)
    return result


def _latlon(ring: Sequence[Point]) -> list[tuple[float, float]]:
    return [(latitude, longitude) for longitude, latitude in ring]


def _cover_land(rh: Any, records: Sequence[Sequence[Ring]]) -> list[str]:
    cells: set[str] = set()
    polygons = [part for record in records for part in _polygon_parts(record)]
    for index, (exterior, holes) in enumerate(polygons, start=1):
        cells.update(
            rh.polygon_to_cells_intersects(
                _latlon(exterior),
                5,
                holes=[_latlon(hole) for hole in holes] or None,
            )
        )
        print(
            f"\rCovering Natural Earth land: {index}/{len(polygons)} polygons",
            end="",
            file=sys.stderr,
            flush=True,
        )
    print(file=sys.stderr)
    return sorted(rh.compact_cells(sorted(cells)), key=lambda cell: (len(cell), cell))


def _unwrap_longitudes(points: Sequence[Point]) -> Ring:
    if not points:
        return []
    result = [points[0]]
    for longitude, latitude in points[1:]:
        previous = result[-1][0]
        while longitude - previous > 180.0:
            longitude -= 360.0
        while longitude - previous < -180.0:
            longitude += 360.0
        result.append((longitude, latitude))
    return result


def _clip_polygon_edge(
    points: Sequence[Point], boundary: float, keep_greater: bool
) -> Ring:
    if not points:
        return []

    def inside(point: Point) -> bool:
        return point[0] >= boundary if keep_greater else point[0] <= boundary

    def intersection(start: Point, end: Point) -> Point:
        if math.isclose(start[0], end[0]):
            return (boundary, start[1])
        ratio = (boundary - start[0]) / (end[0] - start[0])
        return (boundary, start[1] + ratio * (end[1] - start[1]))

    result: Ring = []
    previous = points[-1]
    previous_inside = inside(previous)
    for current in points:
        current_inside = inside(current)
        if current_inside:
            if not previous_inside:
                result.append(intersection(previous, current))
            result.append(current)
        elif previous_inside:
            result.append(intersection(previous, current))
        previous, previous_inside = current, current_inside
    return _open_ring(result)


def _split_polygon_at_antimeridian(points: Sequence[Point]) -> list[Ring]:
    unwrapped = _unwrap_longitudes(points)
    polygons: list[Ring] = []
    seen: set[tuple[tuple[float, float], ...]] = set()
    for shift in (-360.0, 0.0, 360.0):
        shifted = [(longitude + shift, latitude) for longitude, latitude in unwrapped]
        clipped = _clip_polygon_edge(shifted, -180.0, keep_greater=True)
        clipped = _clip_polygon_edge(clipped, 180.0, keep_greater=False)
        if len(clipped) < 3 or abs(_signed_area(clipped)) <= 1e-13:
            continue
        rounded = tuple((_round(x), _round(y)) for x, y in clipped)
        if rounded not in seen:
            seen.add(rounded)
            polygons.append(list(rounded))
    return polygons


def _clip_line_segment(
    start: Point, end: Point
) -> Optional[tuple[Point, Point]]:
    x1, y1 = start
    x2, y2 = end
    if max(x1, x2) < -180.0 or min(x1, x2) > 180.0:
        return None
    if x1 < -180.0:
        ratio = (-180.0 - x1) / (x2 - x1)
        x1, y1 = -180.0, y1 + ratio * (y2 - y1)
    elif x1 > 180.0:
        ratio = (180.0 - x1) / (x2 - x1)
        x1, y1 = 180.0, y1 + ratio * (y2 - y1)
    if x2 < -180.0:
        ratio = (-180.0 - x1) / (x2 - x1)
        x2, y2 = -180.0, y1 + ratio * (y2 - y1)
    elif x2 > 180.0:
        ratio = (180.0 - x1) / (x2 - x1)
        x2, y2 = 180.0, y1 + ratio * (y2 - y1)
    return ((x1, y1), (x2, y2))


def _split_line_at_antimeridian(points: Sequence[Point]) -> list[Ring]:
    values = _unwrap_longitudes((*points, points[0]))
    result: list[Ring] = []
    for shift in (-360.0, 0.0, 360.0):
        shifted = [(longitude + shift, latitude) for longitude, latitude in values]
        current: Ring = []
        for start, end in zip(shifted, shifted[1:]):
            clipped = _clip_line_segment(start, end)
            if clipped is None:
                if len(current) >= 2:
                    result.append(current)
                current = []
                continue
            clipped_start, clipped_end = clipped
            if not current or not _points_equal(current[-1], clipped_start):
                if len(current) >= 2:
                    result.append(current)
                current = [clipped_start]
            current.append(clipped_end)
        if len(current) >= 2:
            result.append(current)

    unique: list[Ring] = []
    seen: set[tuple[tuple[float, float], ...]] = set()
    for line in result:
        rounded = tuple((_round(x), _round(y)) for x, y in line)
        if len(rounded) >= 2 and rounded not in seen:
            seen.add(rounded)
            unique.append(list(rounded))
    return unique


def _split_edge_at_antimeridian(start: Point, end: Point) -> list[tuple[Point, Point]]:
    """Clip one geographic edge into one or two RFC 7946 longitude ranges."""

    unwrapped = _unwrap_longitudes((start, end))
    result: list[tuple[Point, Point]] = []
    seen: set[tuple[Point, Point]] = set()
    for shift in (-360.0, 0.0, 360.0):
        shifted = (
            (unwrapped[0][0] + shift, unwrapped[0][1]),
            (unwrapped[1][0] + shift, unwrapped[1][1]),
        )
        clipped = _clip_line_segment(*shifted)
        if clipped is None:
            continue
        first = (_round(clipped[0][0]), _round(clipped[0][1]))
        second = (_round(clipped[1][0]), _round(clipped[1][1]))
        if first == second:
            continue
        key = (first, second) if first <= second else (second, first)
        if key not in seen:
            seen.add(key)
            result.append(key)
    return result


def _points_equal(first: Point, second: Point) -> bool:
    return math.isclose(first[0], second[0], abs_tol=1e-10) and math.isclose(
        first[1], second[1], abs_tol=1e-10
    )


def _round(value: float) -> float:
    rounded = round(value, 6)
    return 0.0 if rounded == -0.0 else rounded


def _closed_coordinates(ring: Sequence[Point]) -> list[list[float]]:
    values_ring = list(ring)
    # RFC 7946 recommends counter-clockwise exterior rings. Keeping the
    # winding explicit also avoids renderer-dependent globe interiors.
    if _signed_area(values_ring) < 0.0:
        values_ring.reverse()
    values = [
        [_round(longitude), _round(latitude)] for longitude, latitude in values_ring
    ]
    if values[0] != values[-1]:
        values.append(values[0])
    return values


def _line_coordinates(line: Sequence[Point]) -> list[list[float]]:
    return [[_round(longitude), _round(latitude)] for longitude, latitude in line]


def _cap_geometry(latitude: float, north: bool) -> dict[str, Any]:
    base = [[float(longitude), _round(latitude)] for longitude in range(-180, 181, 10)]
    pole = 90.0 if north else -90.0
    if north:
        ring = base + [[180.0, pole], [-180.0, pole], base[0]]
    else:
        ring = list(reversed(base)) + [[-180.0, pole], [180.0, pole], base[-1]]
    return {"type": "Polygon", "coordinates": [ring]}


def _cell_geometry(rh: Any, identifier: str) -> dict[str, Any]:
    cell = rh.WGS84_003.cell(identifier)
    boundary = [
        (float(longitude), float(latitude))
        for longitude, latitude in cell.boundary(n=3, plane=False)
    ]
    if cell.ellipsoidal_shape == "cap":
        latitude = sum(point[1] for point in boundary) / len(boundary)
        return _cap_geometry(latitude, north=latitude > 0.0)

    polygons = _split_polygon_at_antimeridian(boundary)
    if not polygons:
        raise ValueError(f"could not create geographic polygon for cell {identifier}")
    coordinates = [[_closed_coordinates(polygon)] for polygon in polygons]
    if len(coordinates) == 1:
        return {"type": "Polygon", "coordinates": coordinates[0]}
    return {"type": "MultiPolygon", "coordinates": coordinates}


def _cell_collection(rh: Any, cells: Sequence[str]) -> dict[str, Any]:
    return {
        "type": "FeatureCollection",
        "name": "rHEALPix resolution-5 compacted Natural Earth land cover",
        "metadata": {
            "coverage_resolution": 5,
            "cell_count": len(cells),
            "coverage_mode": "intersects",
            "source": "Natural Earth 1:110m land",
        },
        "features": [
            {
                "type": "Feature",
                "properties": {"cell": cell, "resolution": len(cell) - 1},
                "geometry": _cell_geometry(rh, cell),
            }
            for cell in cells
        ],
    }


def _uncompacted_grid_collection(rh: Any, cells: Sequence[str]) -> dict[str, Any]:
    """Return unique r5 boundary edges for the compact cover's exact expansion."""

    raw_cells = sorted(rh.uncompact_cells(list(cells), 5))
    if len(raw_cells) != len(set(raw_cells)):
        raise ValueError("uncompacted cell list contains duplicate identifiers")

    edges: set[tuple[Point, Point]] = set()
    for index, identifier in enumerate(raw_cells, start=1):
        boundary = [
            (float(longitude), float(latitude))
            for latitude, longitude in rh.cell_to_boundary(identifier)
        ]
        for start, end in zip(boundary, (*boundary[1:], boundary[0])):
            edges.update(_split_edge_at_antimeridian(start, end))
        if index % 5_000 == 0 or index == len(raw_cells):
            print(
                f"\rBuilding uncompacted r5 grid: {index:,}/{len(raw_cells):,} cells",
                end="",
                file=sys.stderr,
                flush=True,
            )
    print(file=sys.stderr)

    ordered_edges = sorted(edges)
    return {
        "type": "FeatureCollection",
        "name": "rHEALPix resolution-5 uncompacted Natural Earth land grid",
        "metadata": {
            "coverage_resolution": 5,
            "cell_count": len(raw_cells),
            "unique_edge_count": len(ordered_edges),
            "coverage_mode": "intersects",
            "source": "Natural Earth 1:110m land",
        },
        "features": [
            {
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "MultiLineString",
                    "coordinates": [
                        _line_coordinates(edge) for edge in ordered_edges
                    ],
                },
            }
        ],
    }


def _coast_collection(records: Sequence[Sequence[Ring]]) -> dict[str, Any]:
    lines = [
        _line_coordinates(segment)
        for record in records
        for ring in record
        for segment in _split_line_at_antimeridian(ring)
    ]
    return {
        "type": "FeatureCollection",
        "name": "Natural Earth 1:110m coastlines",
        "features": [
            {
                "type": "Feature",
                "properties": {},
                "geometry": {"type": "MultiLineString", "coordinates": lines},
            }
        ],
    }


def _serialise(value: dict[str, Any]) -> bytes:
    return (json.dumps(value, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def _write_or_check(path: Path, content: bytes, check: bool) -> bool:
    if check:
        return path.is_file() and path.read_bytes() == content
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    print(f"Wrote {path.relative_to(ROOT)} ({len(content) / 1_000_000:.2f} MB)")
    return True


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail if committed GeoJSON is stale"
    )
    arguments = parser.parse_args()

    rh = _load_rhealpix()
    records = _read_polygon_records(SOURCE)
    cells = _cover_land(rh, records)
    counts: dict[int, int] = {}
    for cell in cells:
        counts[len(cell) - 1] = counts.get(len(cell) - 1, 0) + 1
    print(
        f"Compacted to {len(cells):,} cells: "
        + ", ".join(f"r{resolution}={count:,}" for resolution, count in sorted(counts.items()))
    )

    outputs = {
        CELLS_OUTPUT: _serialise(_cell_collection(rh, cells)),
        UNCOMPACTED_OUTPUT: _serialise(_uncompacted_grid_collection(rh, cells)),
        COASTS_OUTPUT: _serialise(_coast_collection(records)),
    }
    stale = [
        path
        for path, content in outputs.items()
        if not _write_or_check(path, content, arguments.check)
    ]
    if stale:
        paths = ", ".join(str(path.relative_to(ROOT)) for path in stale)
        raise SystemExit(f"generated globe data is stale: {paths}")


if __name__ == "__main__":
    main()
