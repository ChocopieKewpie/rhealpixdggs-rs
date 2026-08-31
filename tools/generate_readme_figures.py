#!/usr/bin/env python3
"""Generate the deterministic SVG figures embedded in README.md.

Run after installing the local package. ``--check`` verifies that committed
assets match the current public API and generator byte-for-byte.
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Callable, Dict, Iterable, List, Tuple

import rhealpixdggs as rh

ROOT = Path(__file__).resolve().parents[1]
Point = Tuple[float, float]

STYLE = """
<style>
  :root { color-scheme: light dark; }
  .ink { fill:#172033; } .muted { fill:#526079; } .line { stroke:#27364f; }
  .panel { fill:#f7f9fc; stroke:#c8d2e2; } .grid { stroke:#93a4bd; }
  .blue { fill:#4c78a8; } .teal { fill:#2a9d8f; } .gold { fill:#e9c46a; }
  .coral { fill:#e76f51; } .violet { fill:#7c6bb2; }
  .stroke-blue { stroke:#4c78a8; } .stroke-teal { stroke:#2a9d8f; }
  .stroke-gold { stroke:#c99a27; } .stroke-coral { stroke:#e76f51; }
  .stroke-violet { stroke:#7c6bb2; }
  text { font-family:Inter,Segoe UI,Arial,sans-serif; }
  @media (prefers-color-scheme: dark) {
    .ink { fill:#e6edf3; } .muted { fill:#a8b3c5; } .line { stroke:#c4cedd; }
    .panel { fill:#161b22; stroke:#445168; } .grid { stroke:#66758c; }
  }
</style>
""".strip()


def esc(value: object) -> str:
    return str(value).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def svg(width: int, height: int, body: str, title: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" role="img" aria-labelledby="title">\n'
        f"<title id=\"title\">{esc(title)}</title>\n{STYLE}\n{body}\n</svg>\n"
    )


def text(x: float, y: float, value: object, size: int = 16, cls: str = "ink", anchor: str = "middle", weight: int = 500) -> str:
    return f'<text x="{x:.1f}" y="{y:.1f}" class="{cls}" text-anchor="{anchor}" font-size="{size}" font-weight="{weight}">{esc(value)}</text>'


def rect(x: float, y: float, width: float, height: float, cls: str = "panel", radius: float = 10) -> str:
    return f'<rect x="{x:.1f}" y="{y:.1f}" width="{width:.1f}" height="{height:.1f}" rx="{radius:.1f}" class="{cls}"/>'


def line(x1: float, y1: float, x2: float, y2: float, cls: str = "line", width: float = 2, dash: str = "") -> str:
    dashed = f' stroke-dasharray="{dash}"' if dash else ""
    return f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" class="{cls}" stroke-width="{width}"{dashed}/>'


def polygon(points: Iterable[Point], cls: str, opacity: float = 0.85) -> str:
    values = " ".join(f"{x:.1f},{y:.1f}" for x, y in points)
    return f'<polygon points="{values}" class="{cls}" opacity="{opacity}" stroke="#27364f" stroke-width="1.5"/>'


def polyline(points: Iterable[Point], cls: str = "line", width: float = 2, opacity: float = 1.0) -> str:
    values = " ".join(f"{x:.1f},{y:.1f}" for x, y in points)
    return f'<polyline points="{values}" fill="none" class="{cls}" stroke-width="{width}" opacity="{opacity}" stroke-linejoin="round"/>'


def arrow(x1: float, y1: float, x2: float, y2: float, color: str = "#172033") -> str:
    return f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{color}" stroke-width="3" marker-end="url(#arrowhead)"/>'


FACE_CLASSES = {
    "N": ("violet", "stroke-violet"),
    "O": ("blue", "stroke-blue"),
    "P": ("teal", "stroke-teal"),
    "Q": ("gold", "stroke-gold"),
    "R": ("coral", "stroke-coral"),
    "S": ("violet", "stroke-violet"),
}


def _cells_at_resolution(resolution: int, face: str = "") -> List[str]:
    faces = [face] if face else list("NOPQRS")
    if resolution == 0:
        return faces
    cells: List[str] = []
    for root in faces:
        cells.extend(rh.cell_to_children(root, resolution))
    return cells


def _bounds(points: Iterable[Point]) -> Tuple[float, float, float, float]:
    values = list(points)
    return (
        min(point[0] for point in values),
        max(point[0] for point in values),
        min(point[1] for point in values),
        max(point[1] for point in values),
    )


def _transform(bounds: Tuple[float, float, float, float], panel: Tuple[float, float, float, float], pad: float = 0.0) -> Callable[[Point], Point]:
    x_min, x_max, y_min, y_max = bounds
    panel_x, panel_y, panel_width, panel_height = panel
    scale = min((panel_width - 2 * pad) / (x_max - x_min), (panel_height - 2 * pad) / (y_max - y_min))
    x_offset = panel_x + (panel_width - (x_max - x_min) * scale) / 2
    y_offset = panel_y + (panel_height - (y_max - y_min) * scale) / 2
    return lambda point: (x_offset + (point[0] - x_min) * scale, y_offset + (y_max - point[1]) * scale)


def projection_hierarchy() -> str:
    assert rh.cell_to_children("Q") == [f"Q{digit}" for digit in range(9)]
    assert rh.cell_to_parent("Q381") == "Q38"
    parts = [text(540, 35, "Six faces, one aperture-9 hierarchy", 25, weight=700)]
    size, x0, y0 = 105, 250, 75
    faces = {"N": (0, -1), "O": (0, 0), "P": (1, 0), "Q": (2, 0), "R": (3, 0), "S": (0, 1)}
    colors = {"N": "violet", "O": "blue", "P": "teal", "Q": "gold", "R": "coral", "S": "violet"}
    for face, (column, row) in faces.items():
        x, y = x0 + column * size, y0 + (row + 1) * size
        parts += [rect(x, y, size, size, colors[face], 0), text(x + size / 2, y + 63, face, 26, weight=700)]
    parts += [text(460, 310, "planar rHEALPix arrangement (north_square = south_square = 0)", 14, "muted")]
    panel_x, panel_y, panel_size = 720, 72, 285
    parts.append(rect(panel_x, panel_y, panel_size, panel_size, "panel", 12))
    for level, identifier in enumerate(["Q", "Q3", "Q38", "Q381"]):
        parts.append(text(860, 395 + level * 24, f"resolution {level}: {identifier}", 14, "muted"))
    # Nested 3x3 grids: Q -> Q3 -> Q38 -> Q381.
    x, y, side = panel_x + 18, panel_y + 18, panel_size - 36
    for level, digit in enumerate([3, 8, 1]):
        for i in (1, 2):
            parts.append(line(x + side * i / 3, y, x + side * i / 3, y + side, "grid", 1.4))
            parts.append(line(x, y + side * i / 3, x + side, y + side * i / 3, "grid", 1.4))
        row, column = divmod(digit, 3)
        nx, ny, nside = x + column * side / 3, y + row * side / 3, side / 3
        parts.append(rect(nx, ny, nside, nside, ["blue", "teal", "coral"][level], 0))
        parts.append(text(nx + nside / 2, ny + nside / 2 + 6, digit, max(11, 22 - level * 5), weight=700))
        x, y, side = nx, ny, nside
    parts += [text(860, 505, "Each digit selects one row-major child", 15, weight=600), text(860, 528, "0 1 2  /  3 4 5  /  6 7 8", 14, "muted")]
    return svg(1080, 560, "\n".join(parts), "rHEALPix faces and aperture-9 hierarchy")


def _projected_boundary(identifier: str, density: int = 2) -> List[Point]:
    boundary = rh.WGS84_003.cell(identifier).boundary(n=density, plane=True)
    return [(float(x), float(y)) for x, y in boundary]


def _draw_projected_grid(
    identifiers: Iterable[str],
    transform: Callable[[Point], Point],
    highlight: str = "",
) -> List[str]:
    parts: List[str] = []
    for identifier in identifiers:
        fill, stroke = FACE_CLASSES[identifier[0]]
        points = [transform(point) for point in _projected_boundary(identifier)]
        opacity = 0.72 if identifier == highlight else 0.20
        width = 2.8 if identifier == highlight else 0.8
        values = " ".join(f"{x:.1f},{y:.1f}" for x, y in points)
        parts.append(
            f'<polygon points="{values}" class="{fill} {stroke}" opacity="{opacity}" '
            f'stroke-width="{width}" stroke-linejoin="round"/>'
        )
    return parts


def projected_grid() -> str:
    roots = _cells_at_resolution(0)
    all_root_points = [point for identifier in roots for point in _projected_boundary(identifier)]
    world_bounds = _bounds(all_root_points)
    parts = [text(620, 34, "The implemented rHEALPix grid in projected metres", 25, weight=700)]

    top_panels = [
        (35.0, 70.0, 555.0, 420.0, 0, "resolution 0 · six root faces"),
        (650.0, 70.0, 555.0, 420.0, 1, "resolution 1 · 54 cells"),
    ]
    for x, y, width, height, resolution, label in top_panels:
        parts.append(rect(x, y, width, height, "panel", 12))
        transform = _transform(world_bounds, (x + 25, y + 45, width - 50, height - 75), 4)
        parts.extend(_draw_projected_grid(_cells_at_resolution(resolution), transform))
        parts.append(text(x + width / 2, y + 27, label, 16, weight=700))
        for face in roots:
            nucleus = rh.WGS84_003.cell(face).nucleus()
            px, py = transform((float(nucleus[0]), float(nucleus[1])))
            parts.append(text(px, py + 5, face, 15 if resolution else 20, weight=700))

    q_boundary = _projected_boundary("Q")
    q_bounds = _bounds(q_boundary)
    parts.append(rect(260, 530, 720, 370, "panel", 12))
    parts.append(text(620, 560, "resolution 2 inside face Q · actual projected cell boundaries", 16, weight=700))
    transform = _transform(q_bounds, (300, 580, 640, 280), 5)
    parts.extend(_draw_projected_grid(_cells_at_resolution(2, "Q"), transform, "Q38"))
    q381 = [transform(point) for point in _projected_boundary("Q381")]
    values = " ".join(f"{x:.1f},{y:.1f}" for x, y in q381)
    parts.append(f'<polygon points="{values}" fill="#e76f51" opacity="0.85" stroke="#172033" stroke-width="2.5"/>')
    nucleus = rh.WGS84_003.cell("Q381").nucleus()
    px, py = transform((float(nucleus[0]), float(nucleus[1])))
    parts += [text(px + 30, py - 8, "Q381", 13, weight=700), text(620, 885, "Q38 is highlighted in gold; its resolution-3 child Q381 is coral.", 14, "muted")]
    parts += [text(620, 940, "Coordinates and polygons come directly from Cell.boundary(plane=True); no illustrative grid geometry is substituted.", 14, "muted")]
    return svg(1240, 970, "\n".join(parts), "Actual projected rHEALPix cells at resolutions zero, one and two")


def _geographic_edge_segments(boundary: List[Point]) -> List[Tuple[Point, Point]]:
    # Python boundaries are (latitude, longitude). Split crossings at ±180°
    # so a GIS-style longitude/latitude view never draws a line across the map.
    lonlats = [(float(longitude), float(latitude)) for latitude, longitude in boundary]
    lonlats.append(lonlats[0])
    segments: List[Tuple[Point, Point]] = []
    for start, end in zip(lonlats, lonlats[1:]):
        start_lon, start_lat = start
        end_lon, end_lat = end
        adjusted_lon = end_lon
        while adjusted_lon - start_lon > 180.0:
            adjusted_lon -= 360.0
        while adjusted_lon - start_lon < -180.0:
            adjusted_lon += 360.0
        if -180.0 <= adjusted_lon <= 180.0:
            segments.append((start, (adjusted_lon, end_lat)))
            continue
        edge = 180.0 if adjusted_lon > 180.0 else -180.0
        fraction = (edge - start_lon) / (adjusted_lon - start_lon)
        crossing_lat = start_lat + fraction * (end_lat - start_lat)
        opposite = -edge
        segments.append((start, (edge, crossing_lat)))
        segments.append(((opposite, crossing_lat), (end_lon, end_lat)))
    return segments


def geographic_faces() -> str:
    parts = [text(620, 35, "WGS84 rHEALPix cell footprints in geographic coordinates", 25, weight=700)]
    panel = (65.0, 72.0, 1110.0, 555.0)
    parts.append(rect(*panel, "panel", 12))
    transform = _transform((-180.0, 180.0, -90.0, 90.0), panel, 25)
    for longitude in range(-180, 181, 30):
        start, end = transform((float(longitude), -90.0)), transform((float(longitude), 90.0))
        parts.append(line(start[0], start[1], end[0], end[1], "grid", 0.8, "4 5"))
        if longitude not in (-180, 180):
            parts.append(text(start[0], panel[1] + panel[3] - 8, f"{longitude}°", 10, "muted"))
    for latitude in range(-60, 61, 30):
        start, end = transform((-180.0, float(latitude))), transform((180.0, float(latitude)))
        parts.append(line(start[0], start[1], end[0], end[1], "grid", 0.8, "4 5"))
        parts.append(text(panel[0] + 32, start[1] - 4, f"{latitude}°", 10, "muted", "start"))

    for identifier in _cells_at_resolution(1):
        _, stroke = FACE_CLASSES[identifier[0]]
        boundary = rh.cell_to_boundary_densified(identifier, points_per_edge=12)
        for start, end in _geographic_edge_segments(boundary):
            a, b = transform(start), transform(end)
            parts.append(line(a[0], a[1], b[0], b[1], stroke, 1.25))

    for identifier in "NOPQRS":
        _, stroke = FACE_CLASSES[identifier]
        boundary = rh.cell_to_boundary_densified(identifier, points_per_edge=24)
        for start, end in _geographic_edge_segments(boundary):
            a, b = transform(start), transform(end)
            parts.append(line(a[0], a[1], b[0], b[1], stroke, 3.0))
    labels = {"N": (0.0, 72.0), "O": (-135.0, 0.0), "P": (-45.0, 0.0), "Q": (45.0, 0.0), "R": (135.0, 0.0), "S": (0.0, -72.0)}
    for identifier, point in labels.items():
        px, py = transform(point)
        parts.append(text(px, py + 6, identifier, 21, weight=700))
    parts += [text(620, 662, "Resolution-1 cells shown in longitude/latitude; root-face boundaries are heavier. Polar cells visibly fold and converge.", 14, "muted"), text(620, 687, "The antimeridian is split at the map edge exactly as a GIS renderer should handle wrapped geometry.", 14, "muted")]
    return svg(1240, 715, "\n".join(parts), "Actual rHEALPix cell footprints on a longitude latitude map")


def _normalise_boundary(boundary: List[Point], x: float, y: float, width: float, height: float) -> List[Point]:
    # Public boundaries are latitude/longitude; unwrap near the first point.
    lons = [point[1] for point in boundary]
    for index in range(1, len(lons)):
        while lons[index] - lons[index - 1] > 180:
            lons[index] -= 360
        while lons[index] - lons[index - 1] < -180:
            lons[index] += 360
    lats = [point[0] for point in boundary]
    min_x, max_x, min_y, max_y = min(lons), max(lons), min(lats), max(lats)
    span_x, span_y = max(max_x - min_x, 1e-12), max(max_y - min_y, 1e-12)
    scale = min(width / span_x, height / span_y)
    offset_x = x + (width - span_x * scale) / 2
    offset_y = y + (height - span_y * scale) / 2
    return [(offset_x + (lon - min_x) * scale, offset_y + (max_y - lat) * scale) for lon, lat in zip(lons, lats)]


def cell_shapes() -> str:
    examples = [("P2", "quad", "blue"), ("N", "cap", "violet"), ("N26", "dart", "coral"), ("S43", "skew_quad", "teal")]
    for identifier, shape, _ in examples:
        assert rh.get_cell_shape(identifier) == shape
    parts = [text(600, 36, "Square subdivision, shape-aware geography", 25, weight=700)]
    for index, (identifier, shape, color) in enumerate(examples):
        x = 28 + index * 292
        parts.append(rect(x, 70, 268, 350, "panel", 12))
        boundary = rh.cell_to_boundary_densified(identifier, points_per_edge=12)
        points = _normalise_boundary(boundary, x + 35, 120, 198, 185)
        parts.append(polygon(points, color))
        parts += [text(x + 134, 98, identifier, 19, weight=700), text(x + 134, 340, shape.replace("_", " "), 18, weight=700), text(x + 134, 368, "geographic boundary", 14, "muted")]
    parts += [text(600, 462, "Every cell begins as a projected square; inverse projection folds polar squares into caps, darts and skew quads.", 15, "muted")]
    return svg(1200, 495, "\n".join(parts), "Geographic rHEALPix cell shapes")


def topology_seams() -> str:
    examples = [
        ("face seam", "R888", "O666", "east / west"),
        ("polar seam", "Q888", "S666", "south / north"),
        ("antimeridian", "R555", "O333", "+180° / −180°"),
    ]
    assert rh.cell_to_neighbor("R888", "right") == "O666"
    assert rh.cell_to_neighbor("Q888", "down") == "S666"
    assert rh.are_neighbor_cells("R555", "O333")
    parts = [text(600, 36, "Topology stays continuous where the map is cut", 25, weight=700)]
    for index, (name, left_id, right_id, note) in enumerate(examples):
        x = 28 + index * 390
        parts.append(rect(x, 72, 364, 280, "panel", 12))
        parts += [text(x + 182, 105, name, 18, weight=700), rect(x + 40, 145, 105, 105, "blue", 7), rect(x + 219, 145, 105, 105, "teal", 7)]
        parts += [text(x + 92, 207, left_id, 18, weight=700), text(x + 271, 207, right_id, 18, weight=700)]
        parts += [line(x + 146, 197, x + 218, 197, "line", 3, "7 5"), text(x + 182, 280, "direct neighbours", 15, weight=600), text(x + 182, 310, note, 13, "muted")]
    ring = rh.grid_ring("N0", 1)
    assert ring == ["N1", "N3", "Q2", "R0"]
    parts += [text(600, 400, "Polar traversal example", 17, weight=700), text(600, 430, "grid_ring('N0', 1) → N1, N3, Q2, R0", 16), text(600, 458, "The graph follows globe adjacency—not visual distance in the unfolded plane.", 14, "muted")]
    return svg(1200, 490, "\n".join(parts), "Topology across seams, poles and antimeridian")


def _longitude_near(longitude: float, anchor: float) -> float:
    return anchor + (longitude - anchor + 180.0) % 360.0 - 180.0


def _local_geographic_boundary(identifier: str, anchor: float) -> List[Point]:
    boundary = rh.cell_to_boundary_densified(identifier, points_per_edge=16)
    result: List[Point] = []
    previous = anchor
    for latitude, longitude in boundary:
        value = _longitude_near(float(longitude), previous)
        result.append((value, float(latitude)))
        previous = value
    return result


def edge_traversal_gis() -> str:
    routes = [
        {
            "title": "equatorial face seam",
            "cells": ["Q54", "Q55", "R33", "R34"],
            "anchor": 90.0,
            "bounds": (64.0, 116.0, -16.0, 16.0),
            "note": "Q → R while moving east",
        },
        {
            "title": "antimeridian",
            "cells": ["R54", "R55", "O33", "O34"],
            "anchor": 180.0,
            "bounds": (154.0, 206.0, -16.0, 16.0),
            "note": "R → O across +180° / −180°",
        },
        {
            "title": "equatorial-to-polar seam",
            "cells": ["Q85", "Q88", "S66", "S67"],
            "anchor": 85.0,
            "bounds": (58.0, 108.0, -60.0, -14.0),
            "note": "quad → dart with rotated directions",
        },
    ]
    assert rh.cell_to_neighbor("Q55", "east", plane=False) == "R33"
    assert rh.cell_to_neighbor("R55", "east", plane=False) == "O33"
    assert rh.cell_to_neighbor("Q88", "south", plane=False) == "S66"
    assert rh.cell_to_neighbor("S66", "west", plane=False) == "S67"

    definitions = [
        '<defs><marker id="arrowhead" markerWidth="9" markerHeight="7" refX="8" refY="3.5" orient="auto"><polygon points="0 0, 9 3.5, 0 7" fill="#172033"/></marker>'
    ]
    for index in range(3):
        definitions.append(f'<clipPath id="map-clip-{index}"><rect x="{35 + index * 405}" y="105" width="370" height="350" rx="8"/></clipPath>')
    definitions.append("</defs>")
    parts = definitions + [text(620, 37, "Edge traversal using actual geographic cell polygons", 25, weight=700)]

    for index, route in enumerate(routes):
        x = 35.0 + index * 405.0
        panel = (x, 105.0, 370.0, 350.0)
        parts += [rect(x - 8, 70, 386, 450, "panel", 12), text(x + 185, 96, route["title"], 17, weight=700)]
        transform = _transform(route["bounds"], panel, 0)
        parts.append(f'<g clip-path="url(#map-clip-{index})">')

        x_min, x_max, y_min, y_max = route["bounds"]
        longitude_start = int(x_min // 10) * 10
        latitude_start = int(y_min // 10) * 10
        for longitude in range(longitude_start, int(x_max) + 11, 10):
            a, b = transform((float(longitude), y_min)), transform((float(longitude), y_max))
            parts.append(line(a[0], a[1], b[0], b[1], "grid", 0.7, "4 5"))
        for latitude in range(latitude_start, int(y_max) + 11, 10):
            a, b = transform((x_min, float(latitude))), transform((x_max, float(latitude)))
            parts.append(line(a[0], a[1], b[0], b[1], "grid", 0.7, "4 5"))

        context = set()
        for identifier in route["cells"]:
            context.update(rh.grid_disk(identifier, 1))
        for identifier in sorted(context):
            points = [transform(point) for point in _local_geographic_boundary(identifier, route["anchor"])]
            values = " ".join(f"{px:.1f},{py:.1f}" for px, py in points)
            parts.append(f'<polygon points="{values}" fill="#93a4bd" opacity="0.10" stroke="#75849a" stroke-width="0.8"/>')

        centroids: List[Point] = []
        for step, identifier in enumerate(route["cells"], start=1):
            fill, stroke = FACE_CLASSES[identifier[0]]
            points = [transform(point) for point in _local_geographic_boundary(identifier, route["anchor"])]
            values = " ".join(f"{px:.1f},{py:.1f}" for px, py in points)
            parts.append(f'<polygon points="{values}" class="{fill} {stroke}" opacity="0.72" stroke-width="2.2"/>')
            latitude, longitude = rh.cell_to_centroid(identifier)
            centroid = transform((_longitude_near(float(longitude), route["anchor"]), float(latitude)))
            centroids.append(centroid)

        for start, end in zip(centroids, centroids[1:]):
            parts.append(arrow(start[0], start[1], end[0], end[1]))
        for step, (identifier, centroid) in enumerate(zip(route["cells"], centroids), start=1):
            parts.append(f'<circle cx="{centroid[0]:.1f}" cy="{centroid[1]:.1f}" r="14" fill="#f7f9fc" stroke="#172033" stroke-width="1.8"/>')
            parts.append(text(centroid[0], centroid[1] + 5, step, 12, weight=700))
            parts.append(text(centroid[0], centroid[1] - 20, identifier, 12, weight=700))
        parts.append("</g>")
        parts += [text(x + 185, 485, " → ".join(route["cells"]), 13, weight=700), text(x + 185, 507, route["note"], 12, "muted")]

    parts += [text(620, 558, "Arrows connect implementation-derived ellipsoidal centroids; surrounding cells come from grid_disk(cell, 1).", 14, "muted"), text(620, 582, "The antimeridian panel is locally unwrapped around 180°, as GIS software does for continuous wrapped geometry.", 14, "muted")]
    return svg(1240, 610, "\n".join(parts), "Actual geographic cells traversed across rHEALPix seams")


def stable_u64() -> str:
    identifier = "Q381"
    value = rh.str_to_int(identifier)
    assert value == 3049 and rh.int_to_str(value) == identifier
    post = rh.cell_to_post_order_index(identifier)
    assert post == 795_604_004_266_974
    max_id = rh.str_to_int("S" + "8" * rh.MAX_RESOLUTION)
    assert max_id == 1_389_765_141_638_879 and max_id.bit_length() == 51
    parts = [text(590, 38, "Canonical cell string ↔ stable u64", 25, weight=700)]
    parts += [rect(35, 78, 1110, 128, "panel", 12), text(90, 120, "Q381", 29, "ink", "start", 700)]
    parts += [text(245, 115, "resolution block", 13, "muted"), text(245, 145, "offset(3) = 546", 18)]
    parts += [text(485, 115, "face", 13, "muted"), text(485, 145, "Q = 3", 18)]
    parts += [text(670, 115, "base-9 digits", 13, "muted"), text(670, 145, "381₉ = 316", 18)]
    parts += [text(930, 115, "result", 13, "muted"), text(930, 150, "3049", 27, "teal", weight=700)]
    parts += [text(590, 245, "id = offset(r) + face × 9ʳ + base9(digits)", 21, weight=700), text(590, 278, "offset(r) = 6 × (9ʳ − 1) / 8", 18, "muted")]
    parts += [rect(35, 315, 535, 145, "panel", 12), text(65, 350, "Face numbers", 16, "ink", "start", 700), text(65, 385, "N=0   O=1   P=2   Q=3   R=4   S=5", 17, "ink", "start"), text(65, 425, "Resolution-major, not a packed bitfield", 14, "muted", "start")]
    parts += [rect(610, 315, 535, 145, "panel", 12), text(640, 350, "Current range", 16, "ink", "start", 700), text(640, 385, "resolutions 0…15 use at most 51 bits", 17, "ink", "start"), text(640, 425, f"post-order is separate (Q381 → {post:,})", 14, "muted", "start")]
    return svg(1180, 490, "\n".join(parts), "Stable rHEALPix u64 encoding")


def grid_traversal() -> str:
    children = rh.cell_to_children("Q4")
    assert children == [f"Q4{digit}" for digit in range(9)]
    ring = rh.grid_ring("Q44", 1)
    assert len(ring) == 4
    parts = [text(1000 / 2, 35, "Hierarchy and edge traversal", 25, weight=700)]
    x0, y0, side = 65, 92, 300
    parts += [rect(35, 65, 360, 370, "panel", 12), text(215, 105, "Q4 and its nine children", 17, weight=700)]
    for row in range(3):
        for column in range(3):
            digit = row * 3 + column
            x, y = x0 + column * side / 3, y0 + 55 + row * side / 3
            parts += [rect(x, y, side / 3, side / 3, "blue" if digit == 4 else "panel", 0), text(x + side / 6, y + side / 6 + 6, f"Q4{digit}", 14, weight=700 if digit == 4 else 500)]
    parts += [rect(430, 65, 535, 370, "panel", 12), text(698, 105, "grid_ring('Q44', 1)", 17, weight=700)]
    positions = [(698, 165), (555, 265), (840, 265), (698, 365)]
    for (x, y), identifier, color in zip(positions, ring, ["teal", "gold", "coral", "violet"]):
        parts += [rect(x - 54, y - 38, 108, 76, color, 9), text(x, y + 6, identifier, 16, weight=700)]
        parts.append(line(698, 265, x, y, "line", 2))
    parts += [rect(644, 227, 108, 76, "blue", 9), text(698, 271, "Q44", 17, weight=700), text(698, 413, "parent/children use the identifier tree; rings use edge adjacency", 14, "muted")]
    return svg(1000, 470, "\n".join(parts), "rHEALPix hierarchy and grid-ring traversal")


FIGURES: Dict[str, Callable[[], str]] = {
    "projected-grid.svg": projected_grid,
    "projection-hierarchy.svg": projection_hierarchy,
    "geographic-faces.svg": geographic_faces,
    "cell-shapes.svg": cell_shapes,
    "topology-seams.svg": topology_seams,
    "edge-traversal-gis.svg": edge_traversal_gis,
    "stable-u64.svg": stable_u64,
    "grid-traversal.svg": grid_traversal,
}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "docs" / "images")
    parser.add_argument("--check", action="store_true")
    arguments = parser.parse_args()
    generated = {name: function() for name, function in FIGURES.items()}
    if arguments.check:
        mismatches = [name for name, value in generated.items() if not (arguments.output_dir / name).exists() or (arguments.output_dir / name).read_text(encoding="utf-8") != value]
        if mismatches:
            raise SystemExit("README figures are stale: " + ", ".join(mismatches))
        print(f"verified {len(generated)} README figures")
        return
    arguments.output_dir.mkdir(parents=True, exist_ok=True)
    for name, value in generated.items():
        (arguments.output_dir / name).write_text(value, encoding="utf-8", newline="\n")
    print(f"wrote {len(generated)} README figures to {arguments.output_dir}")


if __name__ == "__main__":
    main()
