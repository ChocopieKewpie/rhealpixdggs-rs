#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rhealpixdggs::{CellId, RhealpixDggs};

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
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
