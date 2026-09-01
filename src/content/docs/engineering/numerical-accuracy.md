---
title: Numerical accuracy
description: Review projection tolerances, polygon robustness, and conformance testing.
---

The projection uses sixth-order series in the ellipsoid's third flattening
for both geodetic-to-authalic latitude and its inverse. The coefficients are
evaluated in nested form using binary64 arithmetic.

This is the optimized method evaluated by Frane Gilić and Mateo Gašparović in
“Enhancing Authalic Latitude Calculation for the rHEALPix DGGS,” *IEEE Journal
of Selected Topics in Applied Earth Observations and Remote Sensing*, vol. 18,
2025, pp. 12675–12683, DOI
[10.1109/JSTARS.2025.3567839](https://doi.org/10.1109/JSTARS.2025.3567839).
Their study reports substantially better numerical accuracy and lower Python
execution time than the older direct/power-series implementation in
`rhealpixdggs-py`.

The series originate in Charles F. F. Karney, “On auxiliary latitudes,”
*Geodesy and Geodynamics*, 2024, DOI
[10.1016/j.geog.2023.12.002](https://doi.org/10.1016/j.geog.2023.12.002).

## Regression policy

Rust tests compare the forward series with 80-decimal-digit WGS84 references
derived from the exact authalic `q(phi)` definition. Samples cover latitude
near the equator, the rHEALPix polar transition, mid-latitudes, and a point
one microdegree from the pole. The inverse series is checked by round-trip at
the same samples. The current error budget is four binary64 epsilon in radians.

Projection and cell-ID tests separately compare the integrated transformation
against the versioned upstream corpus. This keeps the enhanced numerical path
from changing established aperture-9 identifiers at ordinary coordinates.
