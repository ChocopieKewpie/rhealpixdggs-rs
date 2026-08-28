//! Cross-language coverage tests generated from `rHEALPixDGGS` 0.6.0.

use rhealpixdggs::{Ellipsoid, RhealpixDggs};
use serde::Deserialize;

const COVERAGE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/rhealpixdggs-py-0.6.0/coverage-v1.json"
));

type Point = (f64, f64);

#[derive(Debug, Deserialize)]
struct Corpus {
    counts: Counts,
    point_edges: Vec<PointEdgeCase>,
    regions: Vec<RegionCase>,
    lines: Vec<LineCase>,
    polygons: Vec<PolygonCase>,
}

#[derive(Debug, Deserialize)]
struct Counts {
    point_edges: usize,
    regions: usize,
    lines: usize,
    polygons: usize,
}

#[derive(Debug, Deserialize)]
struct PointEdgeCase {
    longitude: f64,
    latitude: f64,
    resolution: u8,
    cell: String,
}

#[derive(Debug, Deserialize)]
struct RegionCase {
    id: String,
    configuration: [u8; 2],
    resolution: u8,
    upper_left: Point,
    lower_right: Point,
    plane: bool,
    cells: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct LineCase {
    id: String,
    configuration: [u8; 2],
    resolution: u8,
    start: Point,
    end: Point,
    plane: bool,
    cells: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PolygonCase {
    id: String,
    resolution: u8,
    exterior: Vec<Point>,
    holes: Vec<Vec<Point>>,
    cells: Vec<String>,
}

fn corpus() -> Corpus {
    serde_json::from_str(COVERAGE_JSON).expect("coverage corpus is valid JSON")
}

fn dggs(configuration: [u8; 2]) -> RhealpixDggs {
    RhealpixDggs::new(Ellipsoid::wgs84(), configuration[0], configuration[1])
}

#[test]
fn corpus_counts_are_current() {
    let corpus = corpus();
    assert_eq!(corpus.counts.point_edges, corpus.point_edges.len());
    assert_eq!(corpus.counts.regions, corpus.regions.len());
    assert_eq!(corpus.counts.lines, corpus.lines.len());
    assert_eq!(corpus.counts.polygons, corpus.polygons.len());
}

#[test]
fn exact_point_edges_match_upstream_corpus() {
    let dggs = RhealpixDggs::wgs84_003();
    for case in corpus().point_edges {
        assert_eq!(
            dggs.cell_from_lonlat(case.longitude, case.latitude, case.resolution)
                .unwrap()
                .to_string(),
            case.cell
        );
    }
}

#[test]
fn region_coverage_matches_upstream_corpus() {
    for case in corpus().regions {
        let dggs = dggs(case.configuration);
        let rows = if case.plane {
            dggs.cells_from_region_projected(case.resolution, case.upper_left, case.lower_right)
        } else {
            dggs.cells_from_region_lonlat(case.resolution, case.upper_left, case.lower_right)
        }
        .unwrap_or_else(|error| panic!("{}: {error}", case.id));
        let actual: Vec<Vec<_>> = rows
            .into_iter()
            .map(|row| row.into_iter().map(|cell| cell.to_string()).collect())
            .collect();
        assert_eq!(actual, case.cells, "{}", case.id);
    }
}

#[test]
fn line_coverage_matches_upstream_corpus() {
    for case in corpus().lines {
        let dggs = dggs(case.configuration);
        let points = [case.start, case.end];
        let cells = if case.plane {
            dggs.cells_from_polyline_projected(case.resolution, &points)
        } else {
            dggs.cells_from_polyline_lonlat(case.resolution, &points)
        }
        .unwrap_or_else(|error| panic!("{}: {error}", case.id));
        let actual: Vec<_> = cells.into_iter().map(|cell| cell.to_string()).collect();
        assert_eq!(actual, case.cells, "{}", case.id);
    }
}

#[test]
fn polygon_coverage_matches_upstream_corpus() {
    let dggs = RhealpixDggs::wgs84_003();
    for case in corpus().polygons {
        let cells = dggs
            .cells_from_polygon_lonlat(case.resolution, &case.exterior, &case.holes, false)
            .unwrap_or_else(|error| panic!("{}: {error}", case.id));
        let actual: Vec<_> = cells.into_iter().map(|cell| cell.to_string()).collect();
        assert_eq!(actual, case.cells, "{}", case.id);
    }
}
