---
title: Natural Earth land outlines
description: Provenance and regeneration notes for the bundled documentation basemaps.
---

`ne_110m_land.shp` is the Natural Earth 1:110 million land polygon dataset,
version 4.0.0. It is bundled solely to generate the continent outlines in the
README globe cover without network access or optional GIS dependencies.

Natural Earth is a public-domain map dataset created by volunteers and
supported by the North American Cartographic Information Society. The source
was accessed on 2026-08-31 from:

https://www.naturalearthdata.com/downloads/110m-physical-vectors/110m-land/

The figure generator contains a small, read-only parser for the polygon records
in this shapefile. The DBF attributes and spatial index are not required.

`ne_10m_nz_land.geojson` is a New Zealand-only extract of the Natural Earth
1:10 million land polygon dataset, version 5.1.1. It retains the source
vertices without geometric simplification and provides the higher-detail
coastline behind the Waka Kotahi CAS maps. Keeping only New Zealand's main and
subantarctic islands makes the reproducible source asset approximately 125 KB
instead of bundling the complete global shapefile.

The 1:10 million source was accessed on 2026-09-01 from:

https://www.naturalearthdata.com/downloads/10m-physical-vectors/10m-land/

Both Natural Earth datasets are in the public domain.
