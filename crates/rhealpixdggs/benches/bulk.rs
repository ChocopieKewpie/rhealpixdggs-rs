#![allow(missing_docs)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rhealpixdggs::RhealpixDggs;

fn coordinates(count: usize) -> Vec<(f64, f64)> {
    (0..count)
        .map(|index| {
            let longitude = (index as f64 * 0.618_033_988_749_894_8).rem_euclid(360.0) - 180.0;
            let latitude = (index as f64 * 0.414_213_562_373_095).rem_euclid(180.0) - 90.0;
            (longitude, latitude)
        })
        .collect()
}

fn bulk_points(criterion: &mut Criterion) {
    let dggs = RhealpixDggs::wgs84_003();
    let mut group = criterion.benchmark_group("bulk_lonlat_to_cell_r9");
    for count in [256, 4_096, 16_384, 65_536] {
        let points = coordinates(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential", count),
            &points,
            |bench, value| {
                bench.iter(|| {
                    dggs.cells_from_lonlats_bulk(black_box(value), black_box(9), false)
                        .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("parallel", count),
            &points,
            |bench, value| {
                bench.iter(|| {
                    dggs.cells_from_lonlats_bulk(black_box(value), black_box(9), true)
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn bulk_nuclei(criterion: &mut Criterion) {
    let dggs = RhealpixDggs::wgs84_003();
    let mut group = criterion.benchmark_group("bulk_cell_to_lonlat_r9");
    for count in [256, 4_096, 16_384] {
        let cells = dggs
            .cells_from_lonlats_bulk(&coordinates(count), 9, false)
            .unwrap();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential", count),
            &cells,
            |bench, value| {
                bench.iter(|| {
                    dggs.lonlats_from_cells_bulk(black_box(value), false)
                        .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("parallel", count),
            &cells,
            |bench, value| {
                bench.iter(|| {
                    dggs.lonlats_from_cells_bulk(black_box(value), true)
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn bulk_boundaries(criterion: &mut Criterion) {
    let dggs = RhealpixDggs::wgs84_003();
    let mut group = criterion.benchmark_group("bulk_boundary_n4_r9");
    for count in [64, 512, 4_096] {
        let cells = dggs
            .cells_from_lonlats_bulk(&coordinates(count), 9, false)
            .unwrap();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential", count),
            &cells,
            |bench, value| {
                bench.iter(|| {
                    dggs.boundaries_lonlat_bulk(black_box(value), 4, false, false)
                        .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("parallel", count),
            &cells,
            |bench, value| {
                bench.iter(|| {
                    dggs.boundaries_lonlat_bulk(black_box(value), 4, false, true)
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

fn small_bboxes(count: usize) -> Vec<(f64, f64, f64, f64)> {
    coordinates(count)
        .into_iter()
        .map(|(longitude, latitude)| {
            let latitude = latitude.clamp(-89.0, 89.0);
            (
                latitude + 0.000_5,
                latitude - 0.000_5,
                longitude + 0.000_5,
                longitude - 0.000_5,
            )
        })
        .collect()
}

fn bulk_bboxes(criterion: &mut Criterion) {
    let dggs = RhealpixDggs::wgs84_003();
    let mut group = criterion.benchmark_group("bulk_bbox_r9");
    for count in [16, 64, 256] {
        let bboxes = small_bboxes(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("sequential", count),
            &bboxes,
            |bench, value| {
                bench.iter(|| {
                    dggs.cells_from_bboxes_bulk(black_box(value), 9, false)
                        .unwrap()
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("parallel", count),
            &bboxes,
            |bench, value| {
                bench.iter(|| {
                    dggs.cells_from_bboxes_bulk(black_box(value), 9, true)
                        .unwrap()
                });
            },
        );
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(20);
    targets = bulk_points, bulk_nuclei, bulk_boundaries, bulk_bboxes
}
criterion_main!(benches);
