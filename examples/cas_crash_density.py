"""Aggregate public Waka Kotahi CAS crash points into rHEALPix cells."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
from typing import Any

import geopandas as gpd
import matplotlib.pyplot as plt
import numpy as np
import requests
from matplotlib.colors import Normalize

import rhealpixdggs as rh
from rhealpixdggs import geo as rhgeo
from rhealpixdggs import numpy as rhnp

CAS_QUERY_URL = (
    "https://services.arcgis.com/CXBb7LAjgIIdcsPt/arcgis/rest/services/"
    "CAS_Data_Public/FeatureServer/0/query"
)
CAS_FIELDS = (
    "OBJECTID,crashYear,crashSeverity,fatalCount,seriousInjuryCount,"
    "minorInjuryCount,region"
)


def fetch_cas(where: str) -> gpd.GeoDataFrame:
    """Download all CAS points matching an ArcGIS SQL where expression."""

    features: list[dict[str, Any]] = []
    offset = 0
    with requests.Session() as session:
        while True:
            response = session.get(
                CAS_QUERY_URL,
                params={
                    "where": where,
                    "outFields": CAS_FIELDS,
                    "returnGeometry": "true",
                    "outSR": 4326,
                    "orderByFields": "OBJECTID",
                    "resultOffset": offset,
                    "resultRecordCount": 2000,
                    "f": "geojson",
                },
                timeout=60,
            )
            response.raise_for_status()
            payload = response.json()
            page = payload.get("features", [])
            features.extend(page)
            if not page or not payload.get("properties", {}).get(
                "exceededTransferLimit", False
            ):
                break
            offset += len(page)

    if not features:
        raise ValueError(f"CAS query returned no crash points: {where}")
    frame = gpd.GeoDataFrame.from_features(features, crs="EPSG:4326")
    frame = frame.loc[frame.geometry.notna() & ~frame.geometry.is_empty].copy()
    if frame.empty:
        raise ValueError(f"CAS query returned no located crash points: {where}")
    return frame


def aggregate_crashes(
    crashes: gpd.GeoDataFrame,
    resolution: int,
) -> gpd.GeoDataFrame:
    """Index crash points and return one equal-area polygon per occupied cell."""

    coordinates = np.column_stack(
        (crashes.geometry.y.to_numpy(), crashes.geometry.x.to_numpy())
    )
    crashes = crashes.copy()
    count_fields = ["fatalCount", "seriousInjuryCount", "minorInjuryCount"]
    crashes[count_fields] = crashes[count_fields].fillna(0).astype("int64")
    crashes["cell_u64"] = rhnp.latlngs_to_cells(coordinates, resolution)
    summary = (
        crashes.groupby("cell_u64", as_index=False)
        .agg(
            crash_count=("OBJECTID", "size"),
            deaths=("fatalCount", "sum"),
            serious_injuries=("seriousInjuryCount", "sum"),
            minor_injuries=("minorInjuryCount", "sum"),
        )
        .sort_values("cell_u64")
    )
    summary["cell_id"] = [
        rh.int_to_str(int(value)) for value in summary["cell_u64"]
    ]

    cells = rhgeo.cells_to_geodataframe(
        summary["cell_id"],
        points_per_edge=8,
    )
    density = cells.merge(summary, on="cell_id", validate="one_to_one")
    density["crashes_per_km2"] = (
        density["crash_count"] / (density["area_m2"] / 1_000_000.0)
    )
    return density


def _regional_plot_extent(frame: gpd.GeoDataFrame) -> tuple[float, float, float, float]:
    """Return a robust map viewport while retaining all rows in the output data."""

    centroids = frame.geometry.centroid
    quantile = 0.003 if len(frame) >= 350 else 0.0
    x_min, x_max = np.quantile(centroids.x, [quantile, 1.0 - quantile])
    y_min, y_max = np.quantile(centroids.y, [quantile, 1.0 - quantile])
    x_padding = max((x_max - x_min) * 0.06, 2_000.0)
    y_padding = max((y_max - y_min) * 0.06, 2_000.0)
    return (
        float(x_min - x_padding),
        float(x_max + x_padding),
        float(y_min - y_padding),
        float(y_max + y_padding),
    )


def _plot_land(axis: Any, crs: Any, extent: tuple[float, float, float, float]) -> None:
    """Draw the bundled Natural Earth land layer when available."""

    data_dir = Path(__file__).parents[1] / "docs" / "data"
    high_resolution_path = data_dir / "ne_10m_nz_land.geojson"
    fallback_path = data_dir / "ne_110m_land.shp"
    if high_resolution_path.exists():
        land = gpd.read_file(high_resolution_path).to_crs(crs)
    elif fallback_path.exists():
        previous_restore = os.environ.get("SHAPE_RESTORE_SHX")
        os.environ["SHAPE_RESTORE_SHX"] = "YES"
        try:
            land = gpd.read_file(fallback_path).set_crs(4326).to_crs(crs)
        finally:
            if previous_restore is None:
                os.environ.pop("SHAPE_RESTORE_SHX", None)
            else:
                os.environ["SHAPE_RESTORE_SHX"] = previous_restore
    else:
        return
    x_min, x_max, y_min, y_max = extent
    land.cx[x_min:x_max, y_min:y_max].plot(
        ax=axis,
        color="#edf1ed",
        edgecolor="#8da0ad",
        linewidth=0.7,
        zorder=0,
    )


def save_plot(frame: gpd.GeoDataFrame, output: Path, title: str) -> None:
    """Save a projected, documentation-quality crash-density figure."""

    output.parent.mkdir(parents=True, exist_ok=True)
    projected = frame.to_crs(2193)
    extent = _regional_plot_extent(projected)
    figure, axis = plt.subplots(figsize=(10.5, 7.2), facecolor="white")
    axis.set_facecolor("#f4f8fb")
    _plot_land(axis, projected.crs, extent)

    upper_density = max(float(projected["crashes_per_km2"].quantile(0.99)), 1.0)
    projected.plot(
        ax=axis,
        column="crashes_per_km2",
        cmap="magma",
        norm=Normalize(vmin=0.0, vmax=upper_density, clip=True),
        linewidth=0.25,
        edgecolor="#243447",
        zorder=2,
    )
    scalar = plt.cm.ScalarMappable(
        norm=Normalize(vmin=0.0, vmax=upper_density, clip=True),
        cmap="magma",
    )
    colorbar = figure.colorbar(
        scalar,
        ax=axis,
        orientation="horizontal",
        fraction=0.05,
        pad=0.04,
        aspect=35,
    )
    colorbar.set_label("Police-reported crashes per km²")

    x_min, x_max, y_min, y_max = extent
    axis.set_xlim(x_min, x_max)
    axis.set_ylim(y_min, y_max)
    axis.set_aspect("equal")
    axis.set_axis_off()
    axis.set_title(title, loc="left", fontsize=17, fontweight="bold", pad=18)

    resolution = len(str(projected.iloc[0]["cell_id"])) - 1
    area_km2 = float(projected.iloc[0]["area_m2"]) / 1_000_000.0
    crash_count = int(projected["crash_count"].sum())
    axis.text(
        0.0,
        1.015,
        (
            f"{crash_count:,} crashes · resolution {resolution} · "
            f"{area_km2:.3f} km² per cell · {len(projected):,} occupied cells"
        ),
        transform=axis.transAxes,
        color="#526079",
        fontsize=10.5,
        va="bottom",
    )
    axis.text(
        0.995,
        0.012,
        "Waka Kotahi CAS · Natural Earth 1:10m · NZTM2000",
        transform=axis.transAxes,
        color="#526079",
        fontsize=8.5,
        ha="right",
        va="bottom",
        zorder=3,
    )
    figure.subplots_adjust(left=0.04, right=0.98, top=0.88, bottom=0.11)
    figure.savefig(output, dpi=200, facecolor="white")
    plt.close(figure)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--year", type=int, default=2024)
    result.add_argument("--region", default="Wellington Region")
    result.add_argument("--resolution", type=int, default=8)
    result.add_argument("--output", type=Path, default=Path("cas-crash-density.gpkg"))
    result.add_argument("--plot", type=Path)
    return result


def main() -> None:
    arguments = parser().parse_args()
    where = (
        f"crashYear = {arguments.year} AND "
        f"region = '{arguments.region.replace(chr(39), chr(39) * 2)}'"
    )
    crashes = fetch_cas(where)
    print(f"downloaded {len(crashes):,} crash records")
    density = aggregate_crashes(crashes, arguments.resolution)
    rhgeo.write_geopackage(
        density,
        arguments.output,
        layer="cas_crash_density",
        overwrite=True,
    )
    print(f"wrote {len(density):,} occupied cells to {arguments.output}")
    if arguments.plot is not None:
        save_plot(
            density,
            arguments.plot,
            f"{arguments.region} crash density · {arguments.year}",
        )
        print(f"wrote {arguments.plot}")


if __name__ == "__main__":
    main()
