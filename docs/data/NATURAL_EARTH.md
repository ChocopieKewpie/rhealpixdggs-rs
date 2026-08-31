# Natural Earth land outline

`ne_110m_land.shp` is the Natural Earth 1:110 million land polygon dataset,
version 4.0.0. It is bundled solely to generate the continent outlines in the
README globe cover without network access or optional GIS dependencies.

Natural Earth is a public-domain map dataset created by volunteers and
supported by the North American Cartographic Information Society. The source
was accessed on 2026-08-31 from:

https://www.naturalearthdata.com/downloads/110m-physical-vectors/110m-land/

The figure generator contains a small, read-only parser for the polygon records
in this shapefile. The DBF attributes and spatial index are not required.
