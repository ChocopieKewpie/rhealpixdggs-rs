//! Ordered bulk operations with optional Rayon execution.

use std::collections::BTreeSet;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

use crate::cell::{CellId, validate_resolution};
use crate::dggs::{RhealpixDggs, boundary_point_count};
use crate::error::{Error, Result};

const MAX_BULK_BOUNDARY_POINTS: usize = 10_000_000;

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
    /// longitude/latitude pairs.
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
}
