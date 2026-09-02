#!/usr/bin/env python3
"""Build the range-loadable PMTiles archive used by the documentation globe.

The geographic cover remains reproducible through ``generate_globe_data.py``.
This second, deliberately separate step compiles those GeoJSON sources into a
zoom-dependent vector-tile pyramid. It uses a local ``tippecanoe`` executable
when available and otherwise downloads the pinned Node wrapper used by CI.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "docs" / "data"
OUTPUT = DATA / "rhealpix-land-r5.pmtiles"
GRID_OUTPUT = DATA / "rhealpix-land-r5-grid.pmtiles"
TIPPECANOE_PACKAGE = "@bikehopper/node-tippecanoe@0.3.4"
MAX_SOURCE_FEATURE_POSITIONS = 2048

LAYERS = (
    (
        "compact_overview",
        DATA / "rhealpix-land-r5-compacted-render.geojson",
        0,
        1,
    ),
    ("compact_cells", DATA / "rhealpix-land-r5-compacted.geojson", 2, 6),
    ("raw_grid", DATA / "rhealpix-land-r5-uncompacted-grid.geojson", 2, 6),
    ("coast", DATA / "natural-earth-coastlines-110m.geojson", 0, 6),
)

ARCHIVES = (
    (OUTPUT, ("compact_overview", "compact_cells", "coast")),
    (GRID_OUTPUT, ("raw_grid",)),
)


def _tippecanoe_command() -> list[str]:
    executable = shutil.which("tippecanoe")
    if executable:
        return [executable]

    npx_name = "npx.cmd" if os.name == "nt" else "npx"
    npx = shutil.which(npx_name)
    if not npx:
        raise SystemExit(
            "tippecanoe was not found. Install tippecanoe or Node.js/npm; "
            f"the fallback uses `npx {TIPPECANOE_PACKAGE}`."
        )
    return [npx, "--yes", TIPPECANOE_PACKAGE]


def _prepare_layer(
    source: Path, destination: Path, minimum_zoom: int, maximum_zoom: int
) -> None:
    value: dict[str, Any] = json.loads(source.read_text(encoding="utf-8"))
    for index, feature in enumerate(value.get("features", [])):
        position_count = sum(
            1 for _ in _positions(feature.get("geometry", {}).get("coordinates", []))
        )
        if position_count > MAX_SOURCE_FEATURE_POSITIONS:
            raise SystemExit(
                f"{source.relative_to(ROOT)} feature {index} contains "
                f"{position_count:,} coordinate positions; split source features "
                f"below {MAX_SOURCE_FEATURE_POSITIONS:,} positions before tiling"
            )
        feature["tippecanoe"] = {
            "minzoom": minimum_zoom,
            "maxzoom": maximum_zoom,
        }
    destination.write_text(
        json.dumps(value, separators=(",", ":"), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def _positions(value: Any) -> Any:
    if (
        isinstance(value, list)
        and len(value) >= 2
        and isinstance(value[0], (int, float))
        and isinstance(value[1], (int, float))
    ):
        yield value
        return
    if isinstance(value, list):
        for child in value:
            yield from _positions(child)


def _build(work_root: Path) -> list[Path]:
    missing = [path for _, path, _, _ in LAYERS if not path.is_file()]
    if missing:
        names = ", ".join(str(path.relative_to(ROOT)) for path in missing)
        raise SystemExit(
            f"missing generated globe source(s): {names}; "
            "run `python tools/generate_globe_data.py` first"
        )

    staging = work_root / ".pmtiles-build"
    if staging.exists():
        raise SystemExit(f"temporary build directory already exists: {staging}")
    staging.mkdir(parents=True)
    outputs: list[Path] = []
    try:
        prepared_layers: dict[str, str] = {}
        for name, source, minimum_zoom, maximum_zoom in LAYERS:
            prepared = staging / f"{name}.geojson"
            _prepare_layer(source, prepared, minimum_zoom, maximum_zoom)
            prepared_layers[name] = prepared.relative_to(work_root).as_posix()

        for canonical_output, layer_names in ARCHIVES:
            relative_output = canonical_output.relative_to(ROOT)
            output = work_root / relative_output
            output.parent.mkdir(parents=True, exist_ok=True)
            named_layers = [
                f"--named-layer={name}:{prepared_layers[name]}"
                for name in layer_names
            ]
            command = [
                *_tippecanoe_command(),
                "--force",
                f"--output={relative_output.as_posix()}",
                "--name=rHEALPix resolution-5 land grid",
                "--description=Compacted and uncompacted rHEALPix land coverage",
                "--attribution=Grid: rhealpixdggs; coastline: Natural Earth",
                "--projection=EPSG:4326",
                "--minimum-zoom=0",
                "--maximum-zoom=6",
                "--include=cell",
                "--include=resolution",
                "--simplify-only-low-zooms",
                "--no-tiny-polygon-reduction-at-maximum-zoom",
                "--detect-longitude-wraparound",
                "--quiet",
                *named_layers,
            ]
            subprocess.run(command, cwd=work_root, check=True)
            outputs.append(output)
    finally:
        shutil.rmtree(staging)
    return outputs


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="rebuild in a temporary directory and compare with the committed archive",
    )
    arguments = parser.parse_args()

    if arguments.check:
        with tempfile.TemporaryDirectory(prefix="rhealpix-pmtiles-check-") as directory:
            candidates = _build(Path(directory))
            stale = [
                candidate
                for candidate in candidates
                if not (ROOT / candidate.relative_to(directory)).is_file()
                or (ROOT / candidate.relative_to(directory)).read_bytes()
                != candidate.read_bytes()
            ]
            if stale:
                raise SystemExit(
                    "generated globe PMTiles archive is stale; run "
                    "`python tools/build_globe_pmtiles.py`"
                )
        print("Up to date: " + ", ".join(str(path.relative_to(ROOT)) for path, _ in ARCHIVES))
        return

    for output in _build(ROOT):
        print(
            f"Wrote {output.relative_to(ROOT)} "
            f"({output.stat().st_size / 1_000_000:.2f} MB)"
        )


if __name__ == "__main__":
    main()
