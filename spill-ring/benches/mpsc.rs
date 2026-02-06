//! MPSC (Multiple-Producer, Single-Consumer) benchmarks.
//!
//! All multi-threaded benchmarks use the WorkerPool API with pre-warmed rings
//! and monomorphized work functions. This is the recommended path for maximum
//! performance — thread-per-core, no contention on the hot path.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use spill_ring::{MpscRing, SpillRing};
use spout::CollectSpout;
use std::hint::black_box;

/// Benchmark MPSC throughput with varying worker counts.
fn mpsc_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpsc_throughput");

    for num_workers in [1, 2, 4, 8] {
        let iterations_per_worker = 100_000u64;
        let total = iterations_per_worker * num_workers as u64;
        group.throughput(Throughput::Elements(total));

        group.bench_with_input(
            BenchmarkId::new("workers", num_workers),
            &num_workers,
            |b, &n| {
                let mut pool = MpscRing::<u64, 1024>::pool(n).spawn(|ring, id, count: &u64| {
                    for i in 0..*count {
                        ring.push(black_box(id as u64 * 1_000_000 + i));
                    }
                });

                b.iter(|| {
                    pool.run(&iterations_per_worker);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark full cycle: push + drain through consumer.
fn mpsc_full_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpsc_full_cycle");

    for num_workers in [1, 2, 4, 8] {
        let iterations_per_worker = 50_000u64;
        let total = iterations_per_worker * num_workers as u64;
        group.throughput(Throughput::Elements(total));

        group.bench_with_input(
            BenchmarkId::new("workers", num_workers),
            &num_workers,
            |b, &n| {
                let mut pool = MpscRing::<u64, 1024>::pool(n).spawn(|ring, id, count: &u64| {
                    for i in 0..*count {
                        ring.push(black_box(id as u64 * 1_000_000 + i));
                    }
                });

                b.iter(|| {
                    pool.run(&iterations_per_worker);
                });
                // Note: drain happens once after all iterations, not per-iteration.
                // This isolates push throughput from drain cost.
            },
        );
    }
    group.finish();
}

/// Compare single-threaded ring vs pooled 4-worker.
fn mpsc_vs_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpsc_vs_single");

    let total_items = 200_000u64;
    group.throughput(Throughput::Elements(total_items));

    // Single-threaded baseline
    group.bench_function("single_thread", |b| {
        let ring: SpillRing<u64, 1024> = SpillRing::new();
        b.iter(|| {
            for i in 0..total_items {
                ring.push(black_box(i));
            }
        })
    });

    // Pooled 4 workers (50k each)
    group.bench_function("pool_4_workers", |b| {
        let per_worker = total_items / 4;
        let mut pool = MpscRing::<u64, 1024>::pool(4).spawn(|ring, id, count: &u64| {
            for i in 0..*count {
                ring.push(black_box(id as u64 * 1_000_000 + i));
            }
        });

        b.iter(|| {
            pool.run(&per_worker);
        });
    });

    group.finish();
}

/// Benchmark scaling — fixed total work split across workers.
fn mpsc_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpsc_scaling");

    let total_items = 400_000u64;

    for num_workers in [1, 2, 4, 8] {
        let per_worker = total_items / num_workers as u64;
        group.throughput(Throughput::Elements(total_items));

        group.bench_with_input(
            BenchmarkId::new("workers", num_workers),
            &num_workers,
            |b, &n| {
                let mut pool = MpscRing::<u64, 2048>::pool(n).spawn(|ring, id, count: &u64| {
                    for i in 0..*count {
                        ring.push(black_box(id as u64 * 1_000_000 + i));
                    }
                });

                b.iter(|| {
                    pool.run(&per_worker);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark with CollectSpout sink to measure spout overhead.
fn spout_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("spout_overhead");

    let iterations_per_worker = 100_000u64;
    let num_workers = 4;
    let total = iterations_per_worker * num_workers as u64;
    group.throughput(Throughput::Elements(total));

    // DropSpout (no-op sink)
    group.bench_function("drop_spout", |b| {
        let mut pool = MpscRing::<u64, 1024>::pool(num_workers).spawn(|ring, id, count: &u64| {
            for i in 0..*count {
                ring.push(black_box(id as u64 * 1_000_000 + i));
            }
        });

        b.iter(|| {
            pool.run(&iterations_per_worker);
        });
    });

    // CollectSpout (allocating sink)
    group.bench_function("collect_spout", |b| {
        let mut pool =
            MpscRing::<u64, 1024, _>::pool_with_sink(num_workers, CollectSpout::<u64>::new())
                .spawn(|ring, id, count: &u64| {
                    for i in 0..*count {
                        ring.push(black_box(id as u64 * 1_000_000 + i));
                    }
                });

        b.iter(|| {
            pool.run(&iterations_per_worker);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    mpsc_throughput,
    mpsc_full_cycle,
    mpsc_vs_single,
    mpsc_scaling,
    spout_overhead,
);
criterion_main!(benches);
