#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rhealpixdggs::{CellId, Direction, RhealpixDggs};

fn benchmarks(criterion: &mut Criterion) {
    let dggs = RhealpixDggs::wgs84_003();

    criterion.bench_function("latlon_to_cell_r12", |bench| {
        bench.iter(|| {
            dggs.cell_from_lonlat(black_box(175.611), black_box(-40.356), black_box(12))
                .unwrap()
        });
    });

    let cell: CellId = "S407138265401".parse().unwrap();
    criterion.bench_function("cell_to_lonlat_r12", |bench| {
        bench.iter(|| dggs.cell_to_lonlat(black_box(&cell)).unwrap());
    });

    criterion.bench_function("cell_string_roundtrip_r12", |bench| {
        bench.iter(|| {
            let value = black_box("S407138265401");
            value.parse::<CellId>().unwrap().to_string()
        });
    });

    criterion.bench_function("post_order_index_r12", |bench| {
        bench.iter(|| black_box(&cell).post_order_index());
    });

    let post_order_index = cell.post_order_index();
    criterion.bench_function("cell_from_post_order_index_r12", |bench| {
        bench.iter(|| CellId::from_post_order_index(black_box(post_order_index)).unwrap());
    });

    criterion.bench_function("successor_at_r15", |bench| {
        bench.iter(|| black_box(&cell).successor_at(black_box(15)).unwrap());
    });

    let polar_cell: CellId = "N622446670001".parse().unwrap();
    criterion.bench_function("planar_neighbor_polar_r12", |bench| {
        bench.iter(|| dggs.planar_neighbor(black_box(&polar_cell), black_box(Direction::Up)));
    });

    criterion.bench_function("ellipsoidal_neighbors_polar_r12", |bench| {
        bench.iter(|| dggs.ellipsoidal_neighbors(black_box(&polar_cell)).unwrap());
    });

    criterion.bench_function("shape_classification_r12", |bench| {
        bench.iter(|| black_box(&polar_cell).shape());
    });

    criterion.bench_function("geographic_vertices_polar_r12", |bench| {
        bench.iter(|| {
            dggs.cell_vertices_lonlat(black_box(&polar_cell), black_box(false))
                .unwrap()
        });
    });

    criterion.bench_function("projected_boundary_n16_r12", |bench| {
        bench.iter(|| {
            dggs.cell_boundary_projected(black_box(&polar_cell), black_box(16), black_box(false))
                .unwrap()
        });
    });

    criterion.bench_function("geographic_boundary_n16_r12", |bench| {
        bench.iter(|| {
            dggs.cell_boundary_lonlat(black_box(&polar_cell), black_box(16), black_box(false))
                .unwrap()
        });
    });
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
