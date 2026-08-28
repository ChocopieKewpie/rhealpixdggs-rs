# Benchmark geometry

`new-zealand.geojson` is the realistic Rust-scale polygon benchmark. It contains
one WGS84 MultiPolygon with 17 polygon parts and 4,149 coordinate vertices.
The data was downloaded from [SimpleMaps' New Zealand GIS map][simplemaps] and
is redistributed under the [Creative Commons Attribution 4.0 International
license][cc-by-4.0]. Copyright and attribution remain with SimpleMaps.

The Rust-scale benchmark uses this geometry at rHEALPix resolution 8. It is a
simplified national boundary suitable for repeatable software benchmarking,
not an authoritative coastline or cadastral dataset. Upstream 0.6.0 is too
slow for this workload to serve as a practical development benchmark.

`new-zealand-simplified.geojson` is a deterministic, hand-simplified
MultiPolygon for exercising polygon coverage and vector output. It includes
the North Island, South Island, Stewart Island/Rakiura, and a small Chatham
Islands component on the opposite side of the antimeridian. This is the
default cross-implementation fixture at resolution 6; it is small enough for
the pure-Python 0.6.0 reference while still producing 1,859 cells.

It is a software fixture, not an authoritative coastline or administrative
boundary, and must not be used for analysis or cartography. Supply an official
polygon to the converter for real work.

[simplemaps]: https://simplemaps.com/gis/country/nz
[cc-by-4.0]: https://creativecommons.org/licenses/by/4.0/
