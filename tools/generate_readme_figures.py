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
    "projection-hierarchy.svg": projection_hierarchy,
    "cell-shapes.svg": cell_shapes,
    "topology-seams.svg": topology_seams,
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
