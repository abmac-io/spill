//! SPSC (Single-Producer, Single-Consumer) concurrent benchmarks.
//!
//! Rings are pre-warmed once via `Arc<SpillRing>::new()` and reused across
//! iterations by draining remaining items. Thread spawning overhead is included in
//! measurement since it's inherent to SPSC usage, but cache warming is not.
//!
//! NOTE: These benchmarks require the `atomics` feature for thread-safe
//! push/pop. Without it, concurrent access is undefined behavior.
//!
//! API improvement opportunity: a `WorkerPool`-style SPSC harness (persistent
//! producer + consumer threads coordinated by barriers) would eliminate thread
//! spawn overhead from measurement, similar to what `mpsc.rs` benchmarks do.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use spill_ring::SpillRing;
use std::hint::black_box;
use std::{sync::Arc, thread};

/// Benchmark SPSC throughput with varying buffer sizes.
fn spsc_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("spsc_throughput");

    for capacity in [16, 64, 256] {
        let iterations = 100_000u64;
        group.throughput(Throughput::Elements(iterations));
        group.bench_with_input(
            BenchmarkId::from_parameter(capacity),
            &capacity,
            |b, &cap| match cap {
                16 => {
                    let ring = Arc::new(SpillRing::<u64, 16>::new());
                    b.iter(|| spsc_run(&ring, iterations));
                }
                64 => {
                    let ring = Arc::new(SpillRing::<u64, 64>::new());
                    b.iter(|| spsc_run(&ring, iterations));
                }
                256 => {
                    let ring = Arc::new(SpillRing::<u64, 256>::new());
                    b.iter(|| spsc_run(&ring, iterations));
                }
                _ => unreachable!(),
            },
        );
    }
    group.finish();
}

fn spsc_run<const N: usize>(ring: &Arc<SpillRing<u64, N>>, iterations: u64) -> u64 {
    while ring.pop().is_some() {}

    let producer_ring = Arc::clone(ring);
    let producer = thread::spawn(move || {
        for i in 0..iterations {
            producer_ring.push(black_box(i));
        }
    });

    let consumer_ring = Arc::clone(ring);
    let consumer = thread::spawn(move || {
        let mut count = 0u64;
        let mut spins = 0;
        loop {
            if let Some(v) = consumer_ring.pop() {
                black_box(v);
                count += 1;
                spins = 0;
            } else {
                spins += 1;
                if spins > 10_000 {
                    break;
                }
                std::hint::spin_loop();
            }
        }
        count
    });

    producer.join().unwrap();
    consumer.join().unwrap()
}

/// Benchmark SPSC with producer faster than consumer (backpressure).
fn spsc_producer_faster(c: &mut Criterion) {
    let mut group = c.benchmark_group("spsc_producer_faster");

    let iterations = 50_000u64;
    group.throughput(Throughput::Elements(iterations));

    group.bench_function("cap_64", |b| {
        let ring = Arc::new(SpillRing::<u64, 64>::new());
        b.iter(|| {
            while ring.pop().is_some() {}

            let producer_ring = Arc::clone(&ring);
            let producer = thread::spawn(move || {
                for i in 0..iterations {
                    producer_ring.push(black_box(i));
                    // Producer is fast - no delay
                }
            });

            let consumer_ring = Arc::clone(&ring);
            let consumer = thread::spawn(move || {
                let mut count = 0u64;
                let mut spins = 0;
                loop {
                    if let Some(v) = consumer_ring.pop() {
                        black_box(v);
                        count += 1;
                        spins = 0;
                        // Consumer is slow - small delay
                        for _ in 0..10 {
                            std::hint::spin_loop();
                        }
                    } else {
                        spins += 1;
                        if spins > 10_000 {
                            break;
                        }
                    }
                }
                count
            });

            producer.join().unwrap();
            consumer.join().unwrap()
        })
    });

    group.finish();
}

/// Benchmark SPSC with consumer faster than producer (no backpressure).
fn spsc_consumer_faster(c: &mut Criterion) {
    let mut group = c.benchmark_group("spsc_consumer_faster");

    let iterations = 50_000u64;
    group.throughput(Throughput::Elements(iterations));

    group.bench_function("cap_64", |b| {
        let ring = Arc::new(SpillRing::<u64, 64>::new());
        b.iter(|| {
            while ring.pop().is_some() {}

            let producer_ring = Arc::clone(&ring);
            let producer = thread::spawn(move || {
                for i in 0..iterations {
                    producer_ring.push(black_box(i));
                    // Producer is slow
                    for _ in 0..10 {
                        std::hint::spin_loop();
                    }
                }
            });

            let consumer_ring = Arc::clone(&ring);
            let consumer = thread::spawn(move || {
                let mut count = 0u64;
                let mut spins = 0;
                loop {
                    if let Some(v) = consumer_ring.pop() {
                        black_box(v);
                        count += 1;
                        spins = 0;
                        // Consumer is fast - no delay
                    } else {
                        spins += 1;
                        if spins > 50_000 {
                            break;
                        }
                        std::hint::spin_loop();
                    }
                }
                count
            });

            producer.join().unwrap();
            consumer.join().unwrap()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    spsc_throughput,
    spsc_producer_faster,
    spsc_consumer_faster,
);
criterion_main!(benches);
