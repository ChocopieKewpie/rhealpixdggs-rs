//! Cross-language projection, vertex, and identifier facade conformance.

use std::str::FromStr;

use rhealpixdggs::{CellId, Ellipsoid, RhealpixDggs};
use serde_json::Value;

const CORPUS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/rhealpixdggs-py-0.6.0/facade-v1.json"
));

fn point(value: &Value) -> (f64, f64) {
    (
        value[0].as_f64().expect("x is numeric"),
        value[1].as_f64().expect("y is numeric"),
    )
}

fn triple(value: &Value) -> (f64, f64, f64) {
    (
        value[0].as_f64().expect("x is numeric"),
        value[1].as_f64().expect("y is numeric"),
        value[2].as_f64().expect("z is numeric"),
    )
}

fn configured(case: &Value) -> RhealpixDggs {
    RhealpixDggs::new(
        Ellipsoid::wgs84(),
        case["configuration"][0]
            .as_u64()
            .expect("north square is numeric") as u8,
        case["configuration"][1]
            .as_u64()
            .expect("south square is numeric") as u8,
    )
}

fn assert_projected(actual: (f64, f64), expected: (f64, f64), tolerance: f64) {
    assert!(
        (actual.0 - expected.0).abs() <= tolerance,
        "x mismatch: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.1 - expected.1).abs() <= tolerance,
        "y mismatch: {actual:?} != {expected:?}"
    );
}

fn assert_geographic(actual: (f64, f64), expected: (f64, f64), tolerance: f64) {
    let longitude_delta = (actual.0 - expected.0 + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        longitude_delta.abs() <= tolerance,
        "longitude mismatch: {actual:?} != {expected:?}"
    );
    assert!(
        (actual.1 - expected.1).abs() <= tolerance,
        "latitude mismatch: {actual:?} != {expected:?}"
    );
}

fn assert_triple(actual: (f64, f64, f64), expected: (f64, f64, f64), tolerance: f64) {
    assert!((actual.0 - expected.0).abs() <= tolerance);
    assert!((actual.1 - expected.1).abs() <= tolerance);
    assert!((actual.2 - expected.2).abs() <= tolerance);
}

#[test]
fn projection_helpers_match_upstream_facade_corpus() {
    let corpus: Value = serde_json::from_str(CORPUS_JSON).expect("valid facade corpus");
    let projected_tolerance = corpus["error_budget"]["projected_absolute_metres"]
        .as_f64()
        .unwrap();
    let geographic_tolerance = corpus["error_budget"]["geographic_absolute_degrees"]
        .as_f64()
        .unwrap();
    for case in corpus["projections"].as_array().unwrap() {
        let dggs = configured(case);
        let input = point(&case["lonlat"]);
        let projected = match case["projection"].as_str().unwrap() {
            "healpix" => dggs.project_healpix_lonlat(input.0, input.1),
            "rhealpix" => dggs.project_lonlat(input.0, input.1),
            value => panic!("unexpected projection {value}"),
        }
        .unwrap();
        assert_projected(projected, point(&case["projected"]), projected_tolerance);
        let roundtrip = match case["projection"].as_str().unwrap() {
            "healpix" => dggs.unproject_healpix_lonlat(projected.0, projected.1),
            "rhealpix" => dggs.unproject_lonlat(projected.0, projected.1),
            _ => unreachable!(),
        }
        .unwrap();
        assert_geographic(roundtrip, point(&case["roundtrip"]), geographic_tolerance);
    }

    for case in corpus["triangle_transforms"].as_array().unwrap() {
        let dggs = configured(case);
        let input = point(&case["healpix"]);
        let transformed = dggs
            .combine_triangles(input.0, input.1, false, None)
            .unwrap();
        assert_projected(transformed, point(&case["rhealpix"]), projected_tolerance);
        assert_projected(
            dggs.combine_triangles(transformed.0, transformed.1, true, None)
                .unwrap(),
            point(&case["roundtrip"]),
            projected_tolerance,
        );
    }
    for case in corpus["triangles"].as_array().unwrap() {
        let dggs = configured(case);
        let input = point(&case["point"]);
        let (number, region) = dggs.triangle(input.0, input.1, true).unwrap();
        assert_eq!(number.map(u64::from), case["number"].as_u64());
        assert_eq!(region.as_str(), case["region"].as_str().unwrap());
    }
    for case in corpus["cartesian"].as_array().unwrap() {
        let dggs = configured(case);
        let lonlat = point(&case["lonlat"]);
        let projected = point(&case["projected"]);
        assert_triple(
            dggs.xyz_lonlat(lonlat.0, lonlat.1).unwrap(),
            triple(&case["xyz_lonlat"]),
            projected_tolerance,
        );
        assert_triple(
            dggs.xyz_projected(projected.0, projected.1).unwrap(),
            triple(&case["xyz_projected"]),
            projected_tolerance,
        );
        assert_triple(
            dggs.xyz_cube_lonlat(lonlat.0, lonlat.1).unwrap(),
            triple(&case["cube_lonlat"]),
            projected_tolerance,
        );
        assert_triple(
            dggs.xyz_cube_projected(projected.0, projected.1).unwrap(),
            triple(&case["cube_projected"]),
            projected_tolerance,
        );
    }
}

#[test]
fn vertex_and_identifier_helpers_match_upstream_facade_corpus() {
    let corpus: Value = serde_json::from_str(CORPUS_JSON).expect("valid facade corpus");
    let projected_tolerance = corpus["error_budget"]["projected_absolute_metres"]
        .as_f64()
        .unwrap();
    let geographic_tolerance = corpus["error_budget"]["geographic_absolute_degrees"]
        .as_f64()
        .unwrap();
    for case in corpus["cells"].as_array().unwrap() {
        let dggs = configured(case);
        let cell = CellId::from_str(case["cell"].as_str().unwrap()).unwrap();
        assert_projected(
            dggs.cell_upper_left_projected(&cell).unwrap(),
            point(&case["upper_left_projected"]),
            projected_tolerance,
        );
        assert_geographic(
            dggs.cell_upper_left_lonlat(&cell).unwrap(),
            point(&case["upper_left_lonlat"]),
            geographic_tolerance,
        );
        assert_projected(
            dggs.cell_northwest_vertex_projected(&cell).unwrap(),
            point(&case["northwest_projected"]),
            projected_tolerance,
        );
        assert_geographic(
            dggs.cell_northwest_vertex_lonlat(&cell).unwrap(),
            point(&case["northwest_lonlat"]),
            geographic_tolerance,
        );
        let (rows, columns) = cell.row_column_digits();
        let expected_rows: Vec<_> = case["row_suid"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .map(|value| value.as_u64().unwrap() as u8)
            .collect();
        let expected_columns: Vec<_> = case["column_suid"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .map(|value| value.as_u64().unwrap() as u8)
            .collect();
        assert_eq!((rows, columns), (expected_rows, expected_columns));
    }

    for case in corpus["overlaps"].as_array().unwrap() {
        let left = CellId::from_str(case["left"].as_str().unwrap()).unwrap();
        let right = CellId::from_str(case["right"].as_str().unwrap()).unwrap();
        assert_eq!(left.overlaps(&right), case["overlaps"].as_bool().unwrap());
    }
}
