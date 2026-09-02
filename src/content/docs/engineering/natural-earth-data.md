---
title: Natural Earth land outlines
description: Provenance and regeneration notes for the bundled documentation basemaps.
---

`ne_110m_land.shp` is the Natural Earth 1:110 million land polygon dataset,
version 4.1.0. It is bundled to generate the continent outlines in the README
cover and the static data behind the interactive documentation globe without
network access or optional GIS dependencies.

Natural Earth is a public-domain map dataset created by volunteers and
supported by the North American Cartographic Information Society. The source
was accessed on 2026-08-31 from:

https://www.naturalearthdata.com/downloads/110m-physical-vectors/110m-land/

The figure and globe-data generators contain small, read-only parsers for the
polygon records in this shapefile. The DBF attributes and spatial index are not
required. `rhealpix-land-r5-compacted.geojson` contains the resolution-5
intersects cover after lossless sibling compaction;
`rhealpix-land-r5-compacted-render.geojson` maps the cover to resolution-3
ancestors and stores each antimeridian-safe polygon part as an independent,
bounded low-zoom feature;
`rhealpix-land-r5-uncompacted-grid.geojson` contains the deduplicated boundary
edges of its exact resolution-5 expansion in bounded, spatially ordered
batches; and
`natural-earth-coastlines-110m.geojson` contains its reference coastline.
`rhealpix-land-r5.pmtiles` packages the compact overview, selectable cells, and
coast into a zoom-dependent vector-tile pyramid.
`rhealpix-land-r5-grid.pmtiles` keeps the larger uncompacted edge layer separate
so the default homepage view never downloads it. Both archives are generated
by `tools/build_globe_pmtiles.py`. `rhealpix-polar-overlay.geojson` is a small
companion layer that preserves cells and coast beyond the ±85.05° Web Mercator
limit used by browser vector tiles.

The bounded source features are intentional. Resolution-wide `MultiPolygon`
or global `MultiLineString` features make a vector archive range-loadable but
still force MapLibre to decode and prepare very large geometries at once.
`tools/build_globe_pmtiles.py` rejects any source feature with more than 2,048
coordinate positions so that this performance regression cannot recur. The
resolution-3 overview is used only at zooms 0–1; exact compacted cells replace
it from zoom 2 onward.

`ne_10m_nz_land.geojson` is a New Zealand-only extract of the Natural Earth
1:10 million land polygon dataset, version 5.1.1. It retains the source
vertices without geometric simplification and provides the higher-detail
coastline behind the Waka Kotahi CAS maps. Keeping only New Zealand's main and
subantarctic islands makes the reproducible source asset approximately 125 KB
instead of bundling the complete global shapefile.

The 1:10 million source was accessed on 2026-09-01 from:

https://www.naturalearthdata.com/downloads/10m-physical-vectors/10m-land/

Both Natural Earth datasets are in the public domain.
