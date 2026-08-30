#![allow(missing_docs)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rhealpixdggs::{CellId, RhealpixDggs};

fn topology_benchmarks(criterion: &mut Criterion) {
    let dggs = RhealpixDggs::wgs84_003();
    let equatorial: CellId = "Q44444444".parse().unwrap();
    let polar: CellId = "N04444444".parse().unwrap();
    let mut group = criterion.benchmark_group("topology");

    for k in [10_u32, 30, 100] {
        let cell_count = dggs.grid_disk(&equatorial, k).unwrap().len() as u64;
        group.throughput(Throughput::Elements(cell_count));
        group.bench_with_input(BenchmarkId::new("grid_disk_equatorial", k), &k, |b, &k| {
            b.iter(|| dggs.grid_disk(&equatorial, k).unwrap())
        });
    }

    for k in [10_u32, 30] {
        let cell_count = dggs.grid_disk(&polar, k).unwrap().len() as u64;
        group.throughput(Throughput::Elements(cell_count));
        group.bench_with_input(BenchmarkId::new("grid_disk_polar", k), &k, |b, &k| {
            b.iter(|| dggs.grid_disk(&polar, k).unwrap())
        });
    }

    group.throughput(Throughput::Elements(1));
    group.bench_function("are_neighbor_cells", |b| {
        let neighbour = dggs.grid_ring(&equatorial, 1).unwrap().remove(0);
        b.iter(|| dggs.are_neighbor_cells(&equatorial, &neighbour).unwrap())
    });
    group.finish();
}

criterion_group!(benches, topology_benchmarks);
criterion_main!(benches);
