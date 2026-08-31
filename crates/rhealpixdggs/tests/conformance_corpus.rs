//! Cross-language conformance tests generated from `rHEALPixDGGS` 0.6.0.

use std::collections::BTreeMap;
use std::str::FromStr;

use rhealpixdggs::{CellId, Direction, Ellipsoid, RhealpixDggs};
use serde::Deserialize;

const CORPUS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/rhealpixdggs-py-0.6.0/conformance-v1.json"
));

#[derive(Debug, Deserialize)]
struct Corpus {
    contract: Contract,
    configurations: Vec<Configuration>,
    point_indexing: Vec<PointCase>,
    cell_geometry: Vec<GeometryCase>,
    topology: Vec<TopologyCase>,
    mixed_post_order: Vec<String>,
    metrics: Vec<MetricCase>,
}

#[derive(Debug, Deserialize)]
struct Contract {
    tolerances: Tolerances,
}

#[derive(Debug, Deserialize)]
struct Tolerances {
    projected_absolute: f64,
    geographic_absolute: f64,
    metric_relative: f64,
}

#[derive(Debug, Deserialize)]
struct Configuration {
    id: String,
    north_square: u8,
    south_square: u8,
}

#[derive(Debug, Deserialize)]
struct PointCase {
    configuration: String,
    lonlat: [f64; 2],
    resolution: u8,
    cell: String,
}

#[derive(Debug, Deserialize)]
struct GeometryCase {
    configuration: String,
    cell: String,
    region: String,
    shape: String,
    nucleus_projected: [f64; 2],
    nucleus_lonlat: [f64; 2],
    vertices_projected: Vec<[f64; 2]>,
    vertices_lonlat: Vec<[f64; 2]>,
    vertices_lonlat_trimmed: Vec<[f64; 2]>,
    boundary_projected_n3: Vec<[f64; 2]>,
    boundary_lonlat_n3: Vec<[f64; 2]>,
    boundary_projected_interior_n3: Vec<[f64; 2]>,
    boundary_lonlat_interior_n3: Vec<[f64; 2]>,
    neighbors_projected: Vec<[String; 2]>,
    neighbors_lonlat: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
struct TopologyCase {
    cell: String,
    level_order_index: u64,
    post_order_index: u64,
    successor: Option<String>,
    predecessor: Option<String>,
    successor_at: BTreeMap<String, Option<String>>,
    predecessor_at: BTreeMap<String, Option<String>>,
    children: Option<Vec<String>>,
    children_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetricCase {
    resolution: u8,
    width_m: f64,
    area_projected_m2: f64,
    area_ellipsoidal_m2: f64,
}

fn corpus() -> Corpus {
    serde_json::from_str(CORPUS_JSON).expect("the checked-in conformance corpus is valid JSON")
}

fn configurations(corpus: &Corpus) -> BTreeMap<&str, RhealpixDggs> {
    corpus
        .configurations
        .iter()
        .map(|configuration| {
            (
                configuration.id.as_str(),
                RhealpixDggs::new(
                    Ellipsoid::wgs84(),
                    configuration.north_square,
                    configuration.south_square,
                ),
            )
        })
        .collect()
}

fn cell_name(cell: Option<CellId>) -> Option<String> {
    cell.map(|cell| cell.to_string())
}

fn assert_close(
    actual: (f64, f64),
    expected: [f64; 2],
    tolerance: f64,
    geographic: bool,
    context: &str,
) {
    let mut difference_x = actual.0 - expected[0];
    if geographic {
        difference_x = (difference_x + 180.0).rem_euclid(360.0) - 180.0;
    }
    assert!(
        difference_x.abs() <= tolerance,
        "{context}: x {} != {} within {tolerance}",
        actual.0,
        expected[0]
    );
    assert!(
        (actual.1 - expected[1]).abs() <= tolerance,
        "{context}: y {} != {} within {tolerance}",
        actual.1,
        expected[1]
    );
}

fn assert_points_close(
    actual: &[(f64, f64)],
    expected: &[[f64; 2]],
    tolerance: f64,
    geographic: bool,
    context: &str,
) {
    assert_eq!(actual.len(), expected.len(), "{context}: point count");
    for (index, (point, reference)) in actual.iter().zip(expected).enumerate() {
        assert_close(
            *point,
            *reference,
            tolerance,
            geographic,
            &format!("{context} point {index}"),
        );
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64, context: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "{context}: {actual} != {expected} within relative tolerance {tolerance}"
    );
}

#[test]
fn point_indexing_matches_upstream_corpus() {
    let corpus = corpus();
    let configurations = configurations(&corpus);

    for case in &corpus.point_indexing {
        let dggs = configurations[case.configuration.as_str()];
        let actual = dggs
            .cell_from_lonlat(case.lonlat[0], case.lonlat[1], case.resolution)
            .unwrap_or_else(|error| panic!("{}: {error}", case.cell));
        assert_eq!(
            actual.to_string(),
            case.cell,
            "{} {:?} resolution {}",
            case.configuration,
            case.lonlat,
            case.resolution
        );
    }
}

#[test]
fn geometry_matches_upstream_corpus() {
    let corpus = corpus();
    let configurations = configurations(&corpus);
    let projected_tolerance = corpus.contract.tolerances.projected_absolute;
    let geographic_tolerance = corpus.contract.tolerances.geographic_absolute;

    for case in &corpus.cell_geometry {
        let dggs = configurations[case.configuration.as_str()];
        let cell = CellId::from_str(&case.cell).expect("fixture cell is valid");
        let context = format!("{} {}", case.configuration, case.cell);

        assert_eq!(cell.region().as_str(), case.region, "{context}: region");
        assert_eq!(cell.shape().as_str(), case.shape, "{context}: shape");
        assert_close(
            dggs.cell_to_projected(&cell).unwrap(),
            case.nucleus_projected,
            projected_tolerance,
            false,
            &format!("{context}: projected nucleus"),
        );
        assert_close(
            dggs.cell_to_lonlat(&cell).unwrap(),
            case.nucleus_lonlat,
            geographic_tolerance,
            true,
            &format!("{context}: geographic nucleus"),
        );

        let vertices_projected = dggs.cell_vertices_projected(&cell).unwrap();
        assert_points_close(
            &vertices_projected,
            &case.vertices_projected,
            projected_tolerance,
            false,
            &format!("{context}: projected vertices"),
        );
        assert_points_close(
            &dggs.cell_vertices_lonlat(&cell, false).unwrap(),
            &case.vertices_lonlat,
            geographic_tolerance,
            true,
            &format!("{context}: geographic vertices"),
        );
        assert_points_close(
            &dggs.cell_vertices_lonlat(&cell, true).unwrap(),
            &case.vertices_lonlat_trimmed,
            geographic_tolerance,
            true,
            &format!("{context}: trimmed vertices"),
        );

        assert_points_close(
            &dggs.cell_boundary_projected(&cell, 3, false).unwrap(),
            &case.boundary_projected_n3,
            projected_tolerance,
            false,
            &format!("{context}: projected boundary"),
        );
        let boundary_lonlat = dggs.cell_boundary_lonlat(&cell, 3, false).unwrap();
        if matches!(case.shape.as_str(), "quad" | "cap") {
            let corners: Vec<_> = boundary_lonlat.iter().step_by(2).copied().collect();
            assert_points_close(
                &corners,
                &case.boundary_lonlat_n3,
                geographic_tolerance,
                true,
                &format!("{context}: corrected geographic boundary corners"),
            );
        } else {
            assert_points_close(
                &boundary_lonlat,
                &case.boundary_lonlat_n3,
                geographic_tolerance,
                true,
                &format!("{context}: geographic boundary"),
            );
        }
        assert_points_close(
            &dggs.cell_boundary_projected(&cell, 3, true).unwrap(),
            &case.boundary_projected_interior_n3,
            projected_tolerance,
            false,
            &format!("{context}: inset projected boundary"),
        );
        let inset_boundary_lonlat = dggs.cell_boundary_lonlat(&cell, 3, true).unwrap();
        if matches!(case.shape.as_str(), "quad" | "cap") {
            assert_eq!(inset_boundary_lonlat.len(), 8);
            assert_eq!(case.boundary_lonlat_interior_n3.len(), 4);
        } else {
            assert_points_close(
                &inset_boundary_lonlat,
                &case.boundary_lonlat_interior_n3,
                geographic_tolerance,
                true,
                &format!("{context}: inset geographic boundary"),
            );
        }

        let projected_neighbors: Vec<_> = Direction::ALL
            .into_iter()
            .map(|direction| {
                [
                    direction.as_str().to_owned(),
                    dggs.planar_neighbor(&cell, direction).to_string(),
                ]
            })
            .collect();
        assert_eq!(
            projected_neighbors, case.neighbors_projected,
            "{context}: projected neighbors"
        );

        let geographic_neighbors: Vec<_> = dggs
            .ellipsoidal_neighbors(&cell)
            .unwrap()
            .into_iter()
            .map(|(direction, neighbor)| [direction.to_string(), neighbor.to_string()])
            .collect();
        assert_eq!(
            geographic_neighbors, case.neighbors_lonlat,
            "{context}: geographic neighbors"
        );
    }
}

#[test]
fn topology_and_traversal_match_upstream_corpus() {
    let corpus = corpus();

    for case in &corpus.topology {
        let cell = CellId::from_str(&case.cell).expect("fixture cell is valid");
        assert_eq!(
            cell.level_order_index(),
            case.level_order_index,
            "{}: level-order index",
            case.cell
        );
        assert_eq!(
            cell.post_order_index(),
            case.post_order_index,
            "{}: post-order index",
            case.cell
        );
        assert_eq!(
            CellId::from_level_order_index(case.level_order_index).unwrap(),
            cell,
            "{}: inverse level-order index",
            case.cell
        );
        assert_eq!(
            CellId::from_post_order_index(case.post_order_index).unwrap(),
            cell,
            "{}: inverse post-order index",
            case.cell
        );
        assert_eq!(cell_name(cell.successor()), case.successor, "{}", case.cell);
        assert_eq!(
            cell_name(cell.predecessor()),
            case.predecessor,
            "{}",
            case.cell
        );

        for (resolution, expected) in &case.successor_at {
            let resolution = resolution.parse().expect("fixture resolution is valid");
            assert_eq!(
                cell_name(cell.successor_at(resolution).unwrap()),
                *expected,
                "{}: successor at {resolution}",
                case.cell
            );
        }
        for (resolution, expected) in &case.predecessor_at {
            let resolution = resolution.parse().expect("fixture resolution is valid");
            assert_eq!(
                cell_name(cell.predecessor_at(resolution).unwrap()),
                *expected,
                "{}: predecessor at {resolution}",
                case.cell
            );
        }

        match (&case.children, &case.children_error) {
            (Some(expected), None) => assert_eq!(
                cell.children()
                    .unwrap()
                    .into_iter()
                    .map(|child| child.to_string())
                    .collect::<Vec<_>>(),
                *expected,
                "{}: children",
                case.cell
            ),
            (None, Some(error)) => {
                assert_eq!(error, "maximum_resolution");
                assert!(
                    cell.children().is_err(),
                    "{}: children must fail",
                    case.cell
                );
            }
            _ => panic!("{}: inconsistent children fixture", case.cell),
        }
    }

    let mut actual: Vec<_> = corpus
        .mixed_post_order
        .iter()
        .map(|identifier| CellId::from_str(identifier).unwrap())
        .collect();
    actual.reverse();
    actual.sort();
    assert_eq!(
        actual
            .into_iter()
            .map(|cell| cell.to_string())
            .collect::<Vec<_>>(),
        corpus.mixed_post_order
    );
}

#[test]
fn metrics_match_upstream_corpus() {
    let corpus = corpus();
    let dggs = RhealpixDggs::wgs84_003();
    let tolerance = corpus.contract.tolerances.metric_relative;

    for case in &corpus.metrics {
        let width = dggs.cell_width(case.resolution).unwrap();
        assert_relative(
            width,
            case.width_m,
            tolerance,
            &format!("resolution {} width", case.resolution),
        );
        assert_relative(
            width * width,
            case.area_projected_m2,
            tolerance,
            &format!("resolution {} projected area", case.resolution),
        );
        assert_relative(
            dggs.cell_area(case.resolution).unwrap(),
            case.area_ellipsoidal_m2,
            tolerance,
            &format!("resolution {} ellipsoidal area", case.resolution),
        );
    }
}
