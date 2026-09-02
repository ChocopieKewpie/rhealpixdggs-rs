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

rHEALPix uses an **aperture-9 hierarchy**. This means that every cell is
subdivided into nine equally sized children at the next resolution.

In projected space, each parent is divided into a 3×3 grid. Children are
numbered in row-major order, starting at the upper-left corner:

| | Column 0<br>left | Column 1<br>centre | Column 2<br>right |
| --- | :---: | :---: | :---: |
| **Row 0 — top** | `0` | `1` | `2` |
| **Row 1 — middle** | `3` | `4` | `5` |
| **Row 2 — bottom** | `6` | `7` | `8` |

The centre child is therefore always digit `4`. For any child digit, its
projected row and column can be calculated as:

- `row = digit // 3`
- `column = digit % 3`

Conversely:

- `digit = 3 × row + column`

Rows increase from top to bottom and columns increase from left to right in
the planar rHEALPix projection. On polar faces, these cells can look rotated,
folded, or distorted when transformed back onto the globe, but their
hierarchical child digits remain defined by this projected 3×3 arrangement.

### Reading a hierarchical cell ID

A cell identifier begins with one of the six root faces—`N`, `O`, `P`, `Q`,
`R`, or `S`—followed by one child digit for every subdivision level.

For example, `Q381` can be read one level at a time:

| Cell | Resolution | Meaning |
| --- | ---: | --- |
| `Q` | 0 | Root face `Q` |
| `Q3` | 1 | Middle-left child of `Q` |
| `Q38` | 2 | Bottom-right child within `Q3` |
| `Q381` | 3 | Top-centre child within `Q38` |

Each digit is interpreted relative to the cell selected by all the preceding
digits. Digit `8` in `Q38`, for example, selects the bottom-right child of
`Q3`—not the bottom-right child of the original root face.

Because identifiers preserve their complete hierarchy, the parent of a cell
can be found by removing its final digit:

- parent of `Q381` → `Q38`
- parent of `Q38` → `Q3`
- parent of `Q3` → `Q`

Likewise, the nine direct children of `Q38` are `Q380` through `Q388`.

### Resolution and cell count

Resolution is the number of child digits after the root-face letter:

- `Q` has no child digits and is resolution 0;
- `Q3` is resolution 1;
- `Q38` is resolution 2;
- `Q381` is resolution 3.

Within one root face, the number of cells at resolution `r` is
9<sup>r</sup>. Because rHEALPix has six root faces, the global number of cells
is:

> **N(r) = 6 × 9<sup>r</sup>**

| Resolution | Cells per root face | Global cells |
| ---: | ---: | ---: |
| 0 | 1 | 6 |
| 1 | 9 | 54 |
| 2 | 81 | 486 |
| 3 | 729 | 4,374 |
| 4 | 6,561 | 39,366 |

The current implementation supports resolutions 0 through 15.
## Equal area does not mean identical map shape

All cells at one resolution have equal ellipsoidal area. Their displayed
shapes differ because a map projection must distort the globe. Equatorial
cells are geographic quadrilaterals; polar cells may be caps, darts, or skew
quadrilaterals.

![Projected and geographic views of quad, cap, dart and skew-quad cells](/rhealpixdggs-rs/images/cell-shapes.svg)

The shape names returned by `get_cell_shape` are `quad`, `cap`, `dart`, and
`skew_quad`.

## Continuous topology

The unfolded plane contains visible cuts, but the cell graph is global. Cells
on opposite sides of a face seam or ±180° longitude can be direct edge
neighbours.

![Topology across seams, poles and the antimeridian](/rhealpixdggs-rs/images/topology-seams.svg)

Use [topology and traversal functions](/rhealpixdggs-rs/api/topology/) rather than inferring
adjacency from a flat rendering.
