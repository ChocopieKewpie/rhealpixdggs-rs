---
title: How the grid works
description: Explore rHEALPix faces, aperture-9 hierarchy, polar geometry, and seams.
---

rHEALPix is a hierarchical equal-area Discrete Global Grid System. It begins
with six root cells and subdivides every parent into nine children.

![The six root faces and aperture-9 hierarchy](/rhealpixdggs-rs/images/projection-hierarchy.svg)

## Six faces

The canonical root cells are `N`, `O`, `P`, `Q`, `R`, and `S`. Four faces form
the equatorial belt. The north and south faces are folded into polar squares in
the planar rHEALPix arrangement.

![Implemented rHEALPix cells in projected metres](/rhealpixdggs-rs/images/projected-grid.svg)

## Aperture-9 subdivision

Each parent is split row-major into a 3×3 grid:

| Digit | Row | Column |
| ---: | ---: | ---: |
| `0` | 0 | 0 |
| `1` | 0 | 1 |
| `2` | 0 | 2 |
| `3` | 1 | 0 |
| `4` | 1 | 1 |
| `5` | 1 | 2 |
| `6` | 2 | 0 |
| `7` | 2 | 1 |
| `8` | 2 | 2 |

`Q381` therefore means root face `Q`, then child `3`, then `8`, then `1`.
Resolution is simply the number of child digits: `Q` is resolution 0 and
`Q381` is resolution 3.

The number of cells at resolution (r) is:

\[
6 \times 9^r
\]

The implementation supports resolutions 0 through 15.

## Equal area does not mean identical map shape

All cells at one resolution have equal ellipsoidal area. Their displayed
shapes differ because a map projection must distort the globe. Equatorial
cells are geographic quadrilaterals; polar cells may be caps, darts, or skew
quadrilaterals.

![Quad, cap, dart and skew-quad geographic cells](/rhealpixdggs-rs/images/cell-shapes.svg)

The shape names returned by `get_cell_shape` are `quad`, `cap`, `dart`, and
`skew_quad`.

## Continuous topology

The unfolded plane contains visible cuts, but the cell graph is global. Cells
on opposite sides of a face seam or ±180° longitude can be direct edge
neighbours.

![Topology across seams, poles and the antimeridian](/rhealpixdggs-rs/images/topology-seams.svg)

Use [topology and traversal functions](/rhealpixdggs-rs/api/topology/) rather than inferring
adjacency from a flat rendering.
