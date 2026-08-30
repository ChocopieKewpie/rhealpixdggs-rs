use std::collections::{BTreeSet, HashSet};

use crate::cell::{CellId, Direction};
use crate::dggs::RhealpixDggs;
use crate::error::{Error, Result};

const MAX_GRID_CELLS: usize = 10_000_000;
// Largest k whose unbounded square-grid disk, 1 + 2k(k + 1), fits the limit.
const MAX_GRID_DISTANCE_AT_LARGE_RESOLUTION: u32 = 2_235;

impl RhealpixDggs {
    /// Return whether two same-resolution cells share an edge.
    ///
    /// A cell is not considered its own neighbour. Polar-square placement is
    /// respected when an edge crosses a resolution-zero face boundary.
    pub fn are_neighbor_cells(&self, origin: &CellId, destination: &CellId) -> Result<bool> {
        ensure_matching_resolutions(origin, destination)?;
        Ok(origin != destination
            && Direction::ALL
                .into_iter()
                .any(|direction| self.planar_neighbor(origin, direction) == *destination))
    }

    /// Return cells whose edge-graph distance from `origin` is at most `k`.
    ///
    /// The origin is first. Remaining cells are grouped by increasing graph
    /// distance, with each distance layer sorted by canonical cell order.
    pub fn grid_disk(&self, origin: &CellId, k: u32) -> Result<Vec<CellId>> {
        let (disk, _) = self.traverse_grid(origin, k, true)?;
        Ok(disk)
    }

    /// Return cells whose edge-graph distance from `origin` is exactly `k`.
    ///
    /// Results are sorted by canonical cell order. `k = 0` returns the origin.
    pub fn grid_ring(&self, origin: &CellId, k: u32) -> Result<Vec<CellId>> {
        let (_, ring) = self.traverse_grid(origin, k, false)?;
        Ok(ring)
    }

    fn traverse_grid(
        &self,
        origin: &CellId,
        k: u32,
        collect_disk: bool,
    ) -> Result<(Vec<CellId>, Vec<CellId>)> {
        validate_grid_size(origin.resolution(), k)?;

        let mut visited = HashSet::from([origin.clone()]);
        let mut frontier = BTreeSet::from([origin.clone()]);
        let mut disk = if collect_disk {
            vec![origin.clone()]
        } else {
            Vec::new()
        };

        for _ in 0..k {
            let mut next = BTreeSet::new();
            for cell in &frontier {
                for direction in Direction::ALL {
                    let neighbour = self.planar_neighbor(cell, direction);
                    if !visited.contains(&neighbour) {
                        next.insert(neighbour);
                    }
                }
            }

            if visited.len() + next.len() > MAX_GRID_CELLS {
                return Err(Error::ExpansionTooLarge(
                    (visited.len() + next.len()) as u64,
                ));
            }

            visited.extend(next.iter().cloned());
            if collect_disk {
                disk.extend(next.iter().cloned());
            }
            frontier = next;
            if frontier.is_empty() {
                break;
            }
        }

        Ok((disk, frontier.into_iter().collect()))
    }
}

fn ensure_matching_resolutions(origin: &CellId, destination: &CellId) -> Result<()> {
    let origin_resolution = origin.resolution();
    let destination_resolution = destination.resolution();
    if origin_resolution == destination_resolution {
        Ok(())
    } else {
        Err(Error::ResolutionMismatch {
            origin: origin_resolution,
            destination: destination_resolution,
        })
    }
}

fn validate_grid_size(resolution: u8, k: u32) -> Result<()> {
    let resolution_cell_count = 6 * 9_u128.pow(u32::from(resolution));
    if resolution_cell_count > MAX_GRID_CELLS as u128 && k > MAX_GRID_DISTANCE_AT_LARGE_RESOLUTION {
        return Err(Error::GridDistanceTooLarge {
            requested: k,
            maximum: MAX_GRID_DISTANCE_AT_LARGE_RESOLUTION,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::ellipsoid::Ellipsoid;

    fn parse(value: &str) -> CellId {
        value.parse().unwrap()
    }

    fn names(cells: Vec<CellId>) -> Vec<String> {
        cells.into_iter().map(|cell| cell.to_string()).collect()
    }

    #[test]
    fn grid_disk_and_ring_use_shortest_edge_distance_layers() {
        let dggs = RhealpixDggs::wgs84_003();
        let origin = parse("Q44");
        let ring_one = dggs.grid_ring(&origin, 1).unwrap();
        let ring_two = dggs.grid_ring(&origin, 2).unwrap();
        let disk_two = dggs.grid_disk(&origin, 2).unwrap();

        assert_eq!(names(ring_one.clone()), ["Q41", "Q43", "Q45", "Q47"]);
        assert_eq!(ring_two.len(), 8);
        assert_eq!(disk_two.len(), 13);
        assert_eq!(disk_two[0], origin);
        assert_eq!(&disk_two[1..5], ring_one.as_slice());
        assert_eq!(&disk_two[5..], ring_two.as_slice());
        assert_eq!(dggs.grid_ring(&origin, 0).unwrap(), [origin.clone()]);
        assert_eq!(dggs.grid_disk(&origin, 0).unwrap(), [origin]);
    }

    #[test]
    fn topology_crosses_antimeridian_and_polar_face_seams() {
        let dggs = RhealpixDggs::wgs84_003();
        let antimeridian = parse("R888");
        let antimeridian_ring = dggs.grid_ring(&antimeridian, 1).unwrap();
        assert!(antimeridian_ring.contains(&parse("O666")));

        let south_seam = parse("Q888");
        let south_ring = dggs.grid_ring(&south_seam, 1).unwrap();
        assert!(south_ring.contains(&parse("S666")));

        let polar = parse("N0");
        assert_eq!(
            names(dggs.grid_ring(&polar, 1).unwrap()),
            ["N1", "N3", "Q2", "R0"]
        );
    }

    #[test]
    fn neighbour_predicate_is_symmetric_across_all_polar_layouts() {
        for north_square in 0..4 {
            for south_square in 0..4 {
                let dggs = RhealpixDggs::new(Ellipsoid::wgs84(), north_square, south_square);
                for origin in [parse("N"), parse("S"), parse("N0"), parse("S43")] {
                    let ring = dggs.grid_ring(&origin, 1).unwrap();
                    let expected: BTreeSet<_> = Direction::ALL
                        .into_iter()
                        .map(|direction| dggs.planar_neighbor(&origin, direction))
                        .collect();
                    assert_eq!(ring, expected.into_iter().collect::<Vec<_>>());
                    for neighbour in ring {
                        assert!(dggs.are_neighbor_cells(&origin, &neighbour).unwrap());
                        assert!(dggs.are_neighbor_cells(&neighbour, &origin).unwrap());
                    }
                    assert!(!dggs.are_neighbor_cells(&origin, &origin).unwrap());
                }
            }
        }
    }

    #[test]
    fn resolution_zero_disk_saturates_the_six_face_graph() {
        let dggs = RhealpixDggs::wgs84_003();
        let origin = parse("N");
        assert_eq!(dggs.grid_disk(&origin, 1).unwrap().len(), 5);
        assert_eq!(dggs.grid_disk(&origin, 2).unwrap().len(), 6);
        assert_eq!(dggs.grid_disk(&origin, 100).unwrap().len(), 6);
        assert!(dggs.grid_ring(&origin, 3).unwrap().is_empty());
    }

    #[test]
    fn neighbour_predicate_requires_matching_resolutions() {
        let dggs = RhealpixDggs::wgs84_003();
        let error = dggs
            .are_neighbor_cells(&parse("Q4"), &parse("Q44"))
            .unwrap_err();
        assert_eq!(
            error,
            Error::ResolutionMismatch {
                origin: 1,
                destination: 2,
            }
        );
    }

    #[test]
    fn unreasonable_grid_expansions_fail_before_traversal() {
        let dggs = RhealpixDggs::wgs84_003();
        let error = dggs.grid_disk(&parse("Q44444444"), 3_000).unwrap_err();
        assert_eq!(
            error,
            Error::GridDistanceTooLarge {
                requested: 3_000,
                maximum: 2_235,
            }
        );
    }
}
