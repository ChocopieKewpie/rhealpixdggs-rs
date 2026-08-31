"""Aggregate public Waka Kotahi CAS crash points into rHEALPix cells."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import geopandas as gpd
import matplotlib.pyplot as plt
import numpy as np
import requests

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


def save_plot(frame: gpd.GeoDataFrame, output: Path, title: str) -> None:
    """Save a simple projected crash-density figure."""

    output.parent.mkdir(parents=True, exist_ok=True)
    axis = frame.to_crs(2193).plot(
        column="crashes_per_km2",
        cmap="magma",
        legend=True,
        linewidth=0.15,
        edgecolor="#243447",
        figsize=(9, 10),
    )
    axis.set_axis_off()
    axis.set_title(title)
    plt.tight_layout()
    plt.savefig(output, dpi=200, bbox_inches="tight")
    plt.close()


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
            f"Police-reported crashes per km² — {arguments.region}, {arguments.year}",
        )
        print(f"wrote {arguments.plot}")


if __name__ == "__main__":
    main()
