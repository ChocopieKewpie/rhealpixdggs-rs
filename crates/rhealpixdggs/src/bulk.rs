//! Ordered bulk operations with optional Rayon execution.

use std::collections::{BTreeSet, HashMap};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::cell::{CellId, Direction, validate_resolution};
use crate::dggs::{RhealpixDggs, boundary_point_count};
use crate::error::{Error, Result};

const MAX_BULK_BOUNDARY_POINTS: usize = 10_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EdgeKey(u64, u64);

impl EdgeKey {
    fn new(left: &CellId, right: &CellId) -> Self {
        let left = left.to_u64();
        let right = right.to_u64();
        if left <= right {
            Self(left, right)
        } else {
            Self(right, left)
        }
    }
}

#[derive(Debug, Clone)]
struct EdgeSpec {
    owner: CellId,
    direction: Direction,
    forward: bool,
}

#[derive(Debug, Clone, Copy)]
struct EdgeUse {
    key: EdgeKey,
    forward: bool,
}

#[derive(Debug, Clone)]
struct BoundaryPlan {
    edges: [EdgeUse; 4],
    northwest_index: usize,
}

/// Default point-count crossover for automatic parallel point conversion.
///
/// Bindings may let callers override this choice. The value is intentionally
/// public so benchmark consumers can report the exact policy they exercised.
pub const POINT_PARALLEL_THRESHOLD: usize = 4_096;

/// Default cell-count crossover for automatic parallel boundary conversion.
pub const BOUNDARY_PARALLEL_THRESHOLD: usize = 512;

/// Default region-count crossover for automatic parallel rectangle coverage.
pub const REGION_PARALLEL_THRESHOLD: usize = 256;

/// Return whether this build includes Rayon-backed bulk execution.
pub const fn parallelism_available() -> bool {
    cfg!(feature = "parallel")
}

impl RhealpixDggs {
    /// Convert ordered longitude/latitude pairs to cells.
    ///
    /// When the `parallel` crate feature and `parallel` argument are both
    /// enabled, work is distributed with Rayon while preserving input order.
    pub fn cells_from_lonlats_bulk(
        &self,
        coordinates: &[(f64, f64)],
        resolution: u8,
        parallel: bool,
    ) -> Result<Vec<CellId>> {
        validate_resolution(resolution)?;
        let _ = parallel;
        #[cfg(feature = "parallel")]
        if parallel {
            return coordinates
                .par_iter()
                .map(|&(longitude, latitude)| {
                    self.cell_from_lonlat(longitude, latitude, resolution)
                })
                .collect();
        }

        coordinates
            .iter()
            .map(|&(longitude, latitude)| self.cell_from_lonlat(longitude, latitude, resolution))
            .collect()
    }

    /// Convert ordered cells to their longitude/latitude nuclei.
    pub fn lonlats_from_cells_bulk(
        &self,
        cells: &[CellId],
        parallel: bool,
    ) -> Result<Vec<(f64, f64)>> {
        let _ = parallel;
        #[cfg(feature = "parallel")]
        if parallel {
            return cells
                .par_iter()
                .map(|cell| self.cell_to_lonlat(cell))
                .collect();
        }

        cells.iter().map(|cell| self.cell_to_lonlat(cell)).collect()
    }

    /// Convert ordered cells to fixed-length geographic boundaries.
    ///
    /// Each inner vector contains exactly `4 * points_per_edge - 4`
    /// longitude/latitude pairs. When `interior` is false, every unique cell
    /// edge is inverse-projected once and reused in reverse for its neighbour.
    /// This makes shared boundaries byte-identical and avoids nearly half the
    /// projection work for dense cell sets. Inset boundaries cannot share
    /// edges and retain the scalar/parallel path.
    pub fn boundaries_lonlat_bulk(
        &self,
        cells: &[CellId],
        points_per_edge: usize,
        interior: bool,
        parallel: bool,
    ) -> Result<Vec<Vec<(f64, f64)>>> {
        let point_count = boundary_point_count(points_per_edge)?;
        let total = cells
            .len()
            .checked_mul(point_count)
            .ok_or(Error::BoundaryTooLarge(u64::MAX))?;
        if total > MAX_BULK_BOUNDARY_POINTS {
            return Err(Error::BoundaryTooLarge(
                u64::try_from(total).unwrap_or(u64::MAX),
            ));
        }
        let _ = parallel;

        if !interior {
            return self.boundaries_lonlat_shared_edges(cells, points_per_edge, parallel);
        }

        #[cfg(feature = "parallel")]
        if parallel {
            return cells
                .par_iter()
                .map(|cell| self.cell_boundary_lonlat(cell, points_per_edge, interior))
                .collect();
        }

        cells
            .iter()
            .map(|cell| self.cell_boundary_lonlat(cell, points_per_edge, interior))
            .collect()
    }

    fn boundaries_lonlat_shared_edges(
        &self,
        cells: &[CellId],
        points_per_edge: usize,
        parallel: bool,
    ) -> Result<Vec<Vec<(f64, f64)>>> {
        let (specs, plans) = self.boundary_edge_plan(cells)?;
        let _ = parallel;

        #[cfg(feature = "parallel")]
        let projected: Vec<(EdgeKey, Vec<(f64, f64)>)> = if parallel {
            specs
                .par_iter()
                .map(|(key, spec)| {
                    self.project_boundary_edge(&spec.owner, spec.direction, points_per_edge)
                        .map(|mut points| {
                            if !spec.forward {
                                points.reverse();
                            }
                            (*key, points)
                        })
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            specs
                .iter()
                .map(|(key, spec)| {
                    self.project_boundary_edge(&spec.owner, spec.direction, points_per_edge)
                        .map(|mut points| {
                            if !spec.forward {
                                points.reverse();
                            }
                            (*key, points)
                        })
                })
                .collect::<Result<Vec<_>>>()?
        };

        #[cfg(not(feature = "parallel"))]
        let projected: Vec<(EdgeKey, Vec<(f64, f64)>)> = specs
            .iter()
            .map(|(key, spec)| {
                self.project_boundary_edge(&spec.owner, spec.direction, points_per_edge)
                    .map(|mut points| {
                        if !spec.forward {
                            points.reverse();
                        }
                        (*key, points)
                    })
            })
            .collect::<Result<Vec<_>>>()?;

        let edges: HashMap<_, _> = projected.into_iter().collect();
        let mut boundaries = Vec::with_capacity(cells.len());
        for plan in plans {
            let mut boundary = Vec::with_capacity(4 * (points_per_edge - 1));
            for edge_use in plan.edges {
                let edge = edges
                    .get(&edge_use.key)
                    .expect("every requested cell edge was projected");
                if edge_use.forward {
                    boundary.extend(edge.iter().take(points_per_edge - 1).copied());
                } else {
                    boundary.extend(edge.iter().rev().take(points_per_edge - 1).copied());
                }
            }
            boundary.rotate_left(plan.northwest_index * (points_per_edge - 1));
            boundaries.push(boundary);
        }
        Ok(boundaries)
    }

    fn boundary_edge_plan(
        &self,
        cells: &[CellId],
    ) -> Result<(HashMap<EdgeKey, EdgeSpec>, Vec<BoundaryPlan>)> {
        let mut specs = HashMap::with_capacity(cells.len().saturating_mul(2).saturating_add(2));
        let mut plans = Vec::with_capacity(cells.len());
        for cell in cells {
            let vertices = self.cell_vertices_projected(cell)?;
            let northwest = self.cell_northwest_vertex_projected(cell)?;
            let northwest_index = vertices
                .iter()
                .position(|point| *point == northwest)
                .expect("the northwest point is one of the planar vertices");
            let mut edge_uses = Vec::with_capacity(4);
            for direction in [
                Direction::Up,
                Direction::Right,
                Direction::Down,
                Direction::Left,
            ] {
                let neighbour = self.planar_neighbor(cell, direction);
                let key = EdgeKey::new(cell, &neighbour);
                let forward = cell.to_u64() == key.0;
                specs.entry(key).or_insert_with(|| EdgeSpec {
                    owner: cell.clone(),
                    direction,
                    forward,
                });
                edge_uses.push(EdgeUse { key, forward });
            }
            plans.push(BoundaryPlan {
                edges: edge_uses
                    .try_into()
                    .expect("every cell has exactly four boundary edges"),
                northwest_index,
            });
        }
        Ok((specs, plans))
    }

    fn project_boundary_edge(
        &self,
        owner: &CellId,
        direction: Direction,
        points_per_edge: usize,
    ) -> Result<Vec<(f64, f64)>> {
        let upper_left = self.cell_upper_left_projected(owner)?;
        let width = self.cell_width(owner.resolution())?;
        let (start, delta) = match direction {
            Direction::Up => (upper_left, (width, 0.0)),
            Direction::Right => ((upper_left.0 + width, upper_left.1), (0.0, -width)),
            Direction::Down => ((upper_left.0 + width, upper_left.1 - width), (-width, 0.0)),
            Direction::Left => ((upper_left.0, upper_left.1 - width), (0.0, width)),
        };
        (0..points_per_edge)
            .map(|index| {
                let fraction = index as f64 / (points_per_edge - 1) as f64;
                self.unproject_lonlat_in_region(
                    start.0 + fraction * delta.0,
                    start.1 + fraction * delta.1,
                    owner.region(),
                )
            })
            .collect()
    }

    /// Cover ordered longitude/latitude bounding boxes with cells.
    ///
    /// Each box is `(north, south, east, west)`. Antimeridian-crossing boxes
    /// use `west > east` and are split automatically. Results are sorted and
    /// de-duplicated within each box, and box order is preserved.
    pub fn cells_from_bboxes_bulk(
        &self,
        bboxes: &[(f64, f64, f64, f64)],
        resolution: u8,
        parallel: bool,
    ) -> Result<Vec<Vec<CellId>>> {
        validate_resolution(resolution)?;
        let _ = parallel;
        #[cfg(feature = "parallel")]
        if parallel {
            return bboxes
                .par_iter()
                .map(|&bbox| self.cells_from_bbox(bbox, resolution))
                .collect();
        }

        bboxes
            .iter()
            .map(|&bbox| self.cells_from_bbox(bbox, resolution))
            .collect()
    }

    fn cells_from_bbox(
        &self,
        (north, south, east, west): (f64, f64, f64, f64),
        resolution: u8,
    ) -> Result<Vec<CellId>> {
        let intervals = if west <= east {
            [(west, east), (0.0, 0.0)]
        } else {
            [(west, 180.0), (-180.0, east)]
        };
        let interval_count = if west <= east { 1 } else { 2 };
        let mut cells = BTreeSet::new();
        for &(interval_west, interval_east) in &intervals[..interval_count] {
            for cell in self
                .cells_from_region_lonlat(
                    resolution,
                    (interval_west, north),
                    (interval_east, south),
                )?
                .into_iter()
                .flatten()
            {
                cells.insert(cell);
            }
        }
        Ok(cells.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulk_point_operations_preserve_order_and_scalar_results() {
        let dggs = RhealpixDggs::wgs84_003();
        let points = [(-180.0, -90.0), (174.7762, -41.2865), (0.0, 90.0)];
        let sequential = dggs.cells_from_lonlats_bulk(&points, 7, false).unwrap();
        let parallel = dggs.cells_from_lonlats_bulk(&points, 7, true).unwrap();
        assert_eq!(parallel, sequential);
        for (point, cell) in points.into_iter().zip(&sequential) {
            assert_eq!(*cell, dggs.cell_from_lonlat(point.0, point.1, 7).unwrap());
        }
        assert_eq!(
            dggs.lonlats_from_cells_bulk(&sequential, true).unwrap(),
            sequential
                .iter()
                .map(|cell| dggs.cell_to_lonlat(cell).unwrap())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn bulk_boundaries_and_antimeridian_boxes_match_scalar_calls() {
        let dggs = RhealpixDggs::wgs84_003();
        let cells = dggs
            .cells_from_lonlats_bulk(&[(174.0, -41.0), (-179.0, 10.0)], 3, false)
            .unwrap();
        let boundaries = dggs.boundaries_lonlat_bulk(&cells, 3, false, true).unwrap();
        assert_eq!(boundaries.len(), 2);
        assert!(boundaries.iter().all(|boundary| boundary.len() == 8));
        assert_eq!(
            boundaries[0],
            dggs.cell_boundary_lonlat(&cells[0], 3, false).unwrap()
        );

        let boxes = [(-40.0, -42.0, 176.0, 173.0), (11.0, 9.0, -178.0, 178.0)];
        let sequential = dggs.cells_from_bboxes_bulk(&boxes, 3, false).unwrap();
        let parallel = dggs.cells_from_bboxes_bulk(&boxes, 3, true).unwrap();
        assert_eq!(parallel, sequential);
        assert_eq!(sequential.len(), boxes.len());
        assert!(!sequential[1].is_empty());
    }

    #[test]
    fn adjacent_bulk_boundaries_project_one_shared_edge_and_reuse_its_bytes() {
        let dggs = RhealpixDggs::wgs84_003();
        let cells = ["Q4".parse().unwrap(), "Q5".parse().unwrap()];
        assert_eq!(dggs.boundary_edge_plan(&cells).unwrap().0.len(), 7);

        let boundaries = dggs
            .boundaries_lonlat_bulk(&cells, 5, false, false)
            .unwrap();
        let right_edge = boundaries[0][4..=8].to_vec();
        let mut left_edge = boundaries[1][12..].to_vec();
        left_edge.push(boundaries[1][0]);
        left_edge.reverse();
        assert_eq!(right_edge, left_edge);
    }

    #[test]
    fn shared_edge_batches_match_scalar_boundaries_across_shapes_and_seams() {
        for north_square in 0..4 {
            for south_square in 0..4 {
                let dggs = RhealpixDggs::new(crate::Ellipsoid::wgs84(), north_square, south_square);
                let cells: Vec<CellId> = ["P2", "N", "N0", "N43", "R888", "O666"]
                    .into_iter()
                    .map(|value| value.parse().unwrap())
                    .collect();
                let boundaries = dggs.boundaries_lonlat_bulk(&cells, 7, false, true).unwrap();
                for (cell, boundary) in cells.iter().zip(boundaries) {
                    let scalar = dggs.cell_boundary_lonlat(cell, 7, false).unwrap();
                    assert_eq!(boundary.len(), scalar.len());
                    for (actual, expected) in boundary.into_iter().zip(scalar) {
                        let longitude_delta =
                            (actual.0 - expected.0 + 180.0).rem_euclid(360.0) - 180.0;
                        assert!(
                            longitude_delta.abs() <= 2.0e-10,
                            "{cell}: {actual:?} {expected:?}"
                        );
                        assert!((actual.1 - expected.1).abs() <= 2.0e-10);
                    }
                }
            }
        }
    }

    #[test]
    fn canonical_edge_keys_deduplicate_the_complete_global_grid() {
        for north_square in 0..4 {
            for south_square in 0..4 {
                let dggs = RhealpixDggs::new(crate::Ellipsoid::wgs84(), north_square, south_square);
                for resolution in 0..=2 {
                    let mut cells = Vec::new();
                    for face in 0..6 {
                        let root = CellId::new(crate::Face::from_number(face).unwrap(), Vec::new())
                            .unwrap();
                        cells.extend(root.descendants(resolution).unwrap());
                    }
                    let (edges, plans) = dggs.boundary_edge_plan(&cells).unwrap();
                    assert_eq!(plans.len(), cells.len());
                    assert_eq!(
                        edges.len(),
                        12 * 9_usize.pow(u32::from(resolution)),
                        "north={north_square} south={south_square} r={resolution}"
                    );
                }
            }
        }
    }
}
