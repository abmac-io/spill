//! Zero-overhead MPSC (Multiple-Producer, Single-Consumer) ring buffer.
//!
//! Each producer owns an independent [`SpillRing`] running at full speed (~4.6 Gelem/s
//! with `no-atomics`). No shared state, no locks, no contention on the hot path.
//! Merging happens only on the cold path when draining.
//!
//! # Example
//!
//! ```
//! use spill_ring_core::{MpscRing, Sink, CollectSink};
//! use std::thread;
//!
//! // Create MPSC with 4 producers
//! let (producers, mut consumer) = MpscRing::<u64, 1024>::new(4);
//!
//! // Each producer runs at full speed on its own thread
//! thread::scope(|s| {
//!     for producer in producers {
//!         s.spawn(move || {
//!             for i in 0..10_000 {
//!                 producer.push(i);
//!             }
//!         });
//!     }
//! });
//!
//! // Cold path: drain all to sink
//! let mut sink = CollectSink::new();
//! consumer.drain(&mut sink);
//! ```

extern crate alloc;

use crate::{DropSink, RingInfo, Sink, SpillRing};
use alloc::vec::Vec;

/// A producer handle for an MPSC ring.
///
/// Each producer owns its own [`SpillRing`] with zero contention.
/// When dropped, remaining items stay in the ring for the consumer to drain.
pub struct Producer<T, const N: usize, S: Sink<T> = DropSink> {
    ring: SpillRing<T, N, S>,
}

impl<T, const N: usize, S: Sink<T>> Producer<T, N, S> {
    /// Push an item to this producer's ring.
    ///
    /// This is the hot path - runs at ~4.6 Gelem/s with `no-atomics`.
    #[inline]
    pub fn push(&self, item: T) {
        self.ring.push(item);
    }

    /// Check if the ring is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// Check if the ring is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.ring.is_full()
    }

    /// Get the number of items in the ring.
    #[inline]
    pub fn len(&self) -> usize {
        self.ring.len()
    }

    /// Get the ring's capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.ring.capacity()
    }
}

impl<T, const N: usize> Producer<T, N, DropSink> {
    fn new() -> Self {
        Self {
            ring: SpillRing::new(),
        }
    }
}

impl<T, const N: usize, S: Sink<T>> Producer<T, N, S> {
    fn with_sink(sink: S) -> Self {
        Self {
            ring: SpillRing::with_sink(sink),
        }
    }

    fn into_ring(self) -> SpillRing<T, N, S> {
        self.ring
    }
}

/// Consumer handle for an MPSC ring.
///
/// Collects all producer rings and provides drain/merge functionality.
/// This is the cold path - only used after producers are done.
pub struct Consumer<T, const N: usize, S: Sink<T> = DropSink> {
    rings: Vec<SpillRing<T, N, S>>,
}

impl<T, const N: usize, S: Sink<T>> Consumer<T, N, S> {
    /// Create a consumer that will collect the given producers.
    fn new() -> Self {
        Self { rings: Vec::new() }
    }

    /// Add a producer's ring to this consumer.
    fn add_ring(&mut self, ring: SpillRing<T, N, S>) {
        self.rings.push(ring);
    }

    /// Drain all items from all rings into a sink.
    ///
    /// Items are drained in producer order, then FIFO within each producer.
    pub fn drain<Sink2: Sink<T>>(&mut self, sink: &mut Sink2) {
        for ring in &mut self.rings {
            sink.send_all(ring.drain());
        }
        sink.flush();
    }

    /// Get the number of producers/rings.
    pub fn num_producers(&self) -> usize {
        self.rings.len()
    }
}

impl<T, const N: usize, S: Sink<T>> RingInfo for Consumer<T, N, S> {
    fn len(&self) -> usize {
        self.rings.iter().map(|r| r.len()).sum()
    }

    fn capacity(&self) -> usize {
        self.rings.iter().map(|r| r.capacity()).sum()
    }
}

/// Zero-overhead MPSC ring buffer.
///
/// Creates independent producers that each own a [`SpillRing`] running at full speed.
/// No shared state, no contention on the hot path.
pub struct MpscRing<T, const N: usize, S: Sink<T> = DropSink> {
    _marker: core::marker::PhantomData<(T, S)>,
}

impl<T, const N: usize> MpscRing<T, N, DropSink> {
    /// Create a new MPSC ring with the given number of producers.
    ///
    /// Returns a vector of producers and a consumer handle.
    /// Each producer owns its own ring - hand them out to separate threads.
    ///
    /// # Example
    ///
    /// ```
    /// use spill_ring_core::MpscRing;
    /// use std::thread;
    ///
    /// let (producers, mut consumer) = MpscRing::<u64, 256>::new(4);
    ///
    /// let handles: Vec<_> = producers
    ///     .into_iter()
    ///     .enumerate()
    ///     .map(|(id, mut p)| {
    ///         thread::spawn(move || {
    ///             for i in 0..1000 {
    ///                 p.push(id as u64 * 10000 + i);
    ///             }
    ///             p  // Return producer so consumer can drain
    ///         })
    ///     })
    ///     .collect();
    ///
    /// // Collect producers back and add to consumer
    /// for handle in handles {
    ///     let producer = handle.join().unwrap();
    ///     // Consumer would need to collect these...
    /// }
    /// ```
    #[allow(clippy::new_ret_no_self)]
    pub fn new(num_producers: usize) -> (Vec<Producer<T, N>>, Consumer<T, N>) {
        let producers: Vec<_> = (0..num_producers).map(|_| Producer::new()).collect();
        let consumer = Consumer::new();
        (producers, consumer)
    }

    /// Create a pre-warmed worker pool with persistent threads.
    ///
    /// This is the recommended API for maximum performance. Each thread owns
    /// its own ring, cache is pre-warmed, and threads are ready before this
    /// returns.
    ///
    /// # Example
    ///
    /// ```
    /// use spill_ring_core::MpscRing;
    ///
    /// let pool = MpscRing::<u64, 1024>::pooled(4);
    /// pool.run(10_000); // Each worker pushes 10k items
    /// let consumer = pool.into_consumer();
    /// ```
    #[cfg(feature = "std")]
    pub fn pooled(num_workers: usize) -> WorkerPool<T, N, DropSink>
    where
        T: Send + 'static,
    {
        WorkerPool::new(num_workers)
    }
}

impl<T, const N: usize, S: Sink<T> + Clone> MpscRing<T, N, S> {
    /// Create a new MPSC ring with the given number of producers and a sink.
    ///
    /// Each producer gets a clone of the sink for handling evictions.
    pub fn with_sink(num_producers: usize, sink: S) -> (Vec<Producer<T, N, S>>, Consumer<T, N, S>) {
        let producers: Vec<_> = (0..num_producers)
            .map(|_| Producer::with_sink(sink.clone()))
            .collect();
        let consumer = Consumer::new();
        (producers, consumer)
    }

    /// Create a pre-warmed worker pool with a custom sink.
    ///
    /// This is the recommended API for maximum performance. Each thread owns
    /// its own ring with a clone of the sink, cache is pre-warmed, and threads
    /// are ready before this returns.
    #[cfg(feature = "std")]
    pub fn pooled_with_sink(num_workers: usize, sink: S) -> WorkerPool<T, N, S>
    where
        T: Send + 'static,
        S: Send + 'static,
    {
        WorkerPool::with_sink(num_workers, sink)
    }
}

/// Collect producers back into a consumer for draining.
///
/// This is a helper to reunite producers with their consumer after threads complete.
pub fn collect_producers<T, const N: usize, S: Sink<T>>(
    producers: impl IntoIterator<Item = Producer<T, N, S>>,
    consumer: &mut Consumer<T, N, S>,
) {
    for producer in producers {
        consumer.add_ring(producer.into_ring());
    }
}

#[cfg(feature = "std")]
mod worker_pool {
    use super::{Consumer, DropSink, Sink, SpillRing, Vec};
    use core::mem::MaybeUninit;
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::sync::{Arc, Barrier};
    use std::thread;

    /// A pool of persistent threads with pre-warmed [`SpillRing`]s.
    ///
    /// Each thread owns its ring for the entire lifetime of the pool.
    /// Threads are spawned once at pool creation and kept alive until dropped.
    ///
    /// The pool is ready to use immediately after construction - all threads
    /// are spawned and cache-warmed before `new()` returns.
    ///
    /// This is a convenience API for simple use cases. For maximum performance,
    /// use the pattern from `mpsc_prewarmed` in the benchmarks which keeps the
    /// work loop in user code where the compiler can fully optimize it.
    pub struct WorkerPool<T, const N: usize, S: Sink<T> = DropSink> {
        cmd_txs: Vec<Sender<u64>>,
        handles: Vec<Option<thread::JoinHandle<SpillRing<T, N, S>>>>,
        start_barrier: Arc<Barrier>,
        done_barrier: Arc<Barrier>,
    }

    impl<T: Send + 'static, const N: usize> WorkerPool<T, N, DropSink> {
        /// Create a new pool with the specified number of persistent threads.
        ///
        /// All threads are spawned and cache-warmed before this returns.
        pub fn new(num_workers: usize) -> Self {
            Self::with_sink(num_workers, DropSink)
        }
    }

    impl<T: Send + 'static, const N: usize, S: Sink<T> + Clone + Send + 'static> WorkerPool<T, N, S> {
        /// Create a new pool with a custom sink for evictions.
        ///
        /// All threads are spawned and cache-warmed before this returns.
        pub fn with_sink(num_workers: usize, sink: S) -> Self {
            assert!(num_workers > 0, "must have at least one worker");

            let ready_barrier = Arc::new(Barrier::new(num_workers + 1));
            let start_barrier = Arc::new(Barrier::new(num_workers + 1));
            let done_barrier = Arc::new(Barrier::new(num_workers + 1));

            let (cmd_txs, cmd_rxs): (Vec<Sender<u64>>, Vec<Receiver<u64>>) =
                (0..num_workers).map(|_| channel()).unzip();

            let handles: Vec<_> = cmd_rxs
                .into_iter()
                .map(|rx| {
                    let sink = sink.clone();
                    let ready = Arc::clone(&ready_barrier);
                    let start = Arc::clone(&start_barrier);
                    let done = Arc::clone(&done_barrier);

                    Some(thread::spawn(move || {
                        let ring = SpillRing::with_sink(sink);

                        // Warm cache by touching all slots with uninitialized writes.
                        // This brings the ring's memory into L1/L2 cache without
                        // requiring T: Default or constructing real values.
                        for i in 0..N {
                            unsafe {
                                let slot = &ring.buffer[i];
                                // Write uninitialized memory to touch the cache line
                                core::ptr::write_volatile(
                                    (*slot.data.get()).as_mut_ptr(),
                                    MaybeUninit::<T>::uninit().assume_init_read(),
                                );
                            }
                        }
                        // Reset indices (no items actually in ring)
                        ring.head.store(0);
                        ring.tail.store(0);

                        // Signal ready, then wait for work
                        ready.wait();

                        while let Ok(count) = rx.recv() {
                            start.wait();
                            for _ in 0..count {
                                unsafe {
                                    // Push uninitialized values - this is a synthetic
                                    // benchmark helper, real usage should push real data
                                    ring.push(MaybeUninit::<T>::uninit().assume_init_read());
                                }
                            }
                            done.wait();
                        }
                        ring
                    }))
                })
                .collect();

            // Wait for all threads to be warmed and ready
            ready_barrier.wait();

            Self {
                cmd_txs,
                handles,
                start_barrier,
                done_barrier,
            }
        }
    }

    impl<T: Send + 'static, const N: usize, S: Sink<T> + Send + 'static> WorkerPool<T, N, S> {
        /// Get the number of workers in the pool.
        #[inline]
        pub fn num_rings(&self) -> usize {
            self.handles.len()
        }

        /// Run work on all rings in parallel.
        ///
        /// Each worker pushes `count` items to its ring. Returns after all complete.
        #[inline]
        pub fn run(&self, count: u64) {
            for tx in &self.cmd_txs {
                let _ = tx.send(count);
            }
            self.start_barrier.wait();
            self.done_barrier.wait();
        }

        /// Send work count to all workers without waiting.
        ///
        /// Call `wait_start()` then `wait_done()` to synchronize.
        #[inline]
        pub fn send(&self, count: u64) {
            for tx in &self.cmd_txs {
                let _ = tx.send(count);
            }
        }

        /// Wait for all workers to start.
        #[inline]
        pub fn wait_start(&self) {
            self.start_barrier.wait();
        }

        /// Wait for all workers to complete.
        #[inline]
        pub fn wait_done(&self) {
            self.done_barrier.wait();
        }

        /// Convert the pool into a [`Consumer`] for draining.
        pub fn into_consumer(mut self) -> Consumer<T, N, S> {
            let mut consumer = Consumer::new();
            self.cmd_txs.clear();
            for handle in &mut self.handles {
                if let Some(h) = handle.take() {
                    consumer.add_ring(h.join().unwrap());
                }
            }
            consumer
        }
    }

    impl<T, const N: usize, S: Sink<T>> Drop for WorkerPool<T, N, S> {
        fn drop(&mut self) {
            self.cmd_txs.clear();
            for handle in &mut self.handles {
                if let Some(h) = handle.take() {
                    let _ = h.join();
                }
            }
        }
    }
}

#[cfg(feature = "std")]
pub use worker_pool::WorkerPool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CollectSink;

    extern crate std;
    use std::vec;

    #[test]
    fn basic_mpsc() {
        let (producers, mut consumer) = MpscRing::<u64, 8>::new(2);

        let producers: Vec<_> = producers.into_iter().collect();

        // Producer 0
        producers[0].push(1);
        producers[0].push(2);

        // Producer 1
        producers[1].push(10);
        producers[1].push(20);

        collect_producers(producers, &mut consumer);

        let mut sink = CollectSink::new();
        consumer.drain(&mut sink);
        let items = sink.into_items();
        assert_eq!(items.len(), 4);
        assert!(items.contains(&1));
        assert!(items.contains(&2));
        assert!(items.contains(&10));
        assert!(items.contains(&20));
    }

    #[test]
    fn drain() {
        let (producers, mut consumer) = MpscRing::<u64, 8>::new(2);

        let producers: Vec<_> = producers.into_iter().collect();

        producers[0].push(1);
        producers[0].push(2);
        producers[1].push(10);

        collect_producers(producers, &mut consumer);

        let mut sink = CollectSink::new();
        consumer.drain(&mut sink);

        let items = sink.into_items();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn producer_overflow_to_sink() {
        let sink = CollectSink::new();
        let (producers, mut consumer) = MpscRing::<u64, 4, _>::with_sink(2, sink);

        let producers: Vec<_> = producers.into_iter().collect();

        // Overflow producer 0
        for i in 0..10 {
            producers[0].push(i);
        }

        // Check producer's local sink got evictions
        // (Can't easily check this without exposing internals)

        collect_producers(producers, &mut consumer);

        // Consumer drains what's left in rings
        let mut sink = CollectSink::new();
        consumer.drain(&mut sink);
        let remaining = sink.into_items();
        assert_eq!(remaining.len(), 4); // Only last 4 fit in ring
    }

    #[test]
    fn empty_producers() {
        let (producers, mut consumer) = MpscRing::<u64, 8>::new(4);

        collect_producers(producers, &mut consumer);

        assert!(consumer.is_empty());
        assert_eq!(consumer.len(), 0);
        assert_eq!(consumer.num_producers(), 4);
    }

    #[test]
    fn single_producer() {
        let (mut producers, mut consumer) = MpscRing::<u64, 16>::new(1);

        let producer = producers.pop().unwrap();
        for i in 0..10 {
            producer.push(i);
        }

        collect_producers([producer], &mut consumer);

        let mut sink = CollectSink::new();
        consumer.drain(&mut sink);
        let items = sink.into_items();
        assert_eq!(items, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[cfg(feature = "std")]
    mod worker_pool_tests {
        use super::*;

        #[test]
        fn basic_worker_pool() {
            let pool = WorkerPool::<u64, 64>::new(2);

            pool.run(50);

            let mut consumer = pool.into_consumer();
            let mut sink = CollectSink::new();
            consumer.drain(&mut sink);

            let items = sink.into_items();
            assert_eq!(items.len(), 100); // 2 workers * 50 each
        }

        #[test]
        fn worker_pool_overflow() {
            // Small ring to force overflow
            let pool = WorkerPool::<u64, 8>::new(1);

            pool.run(100);

            let mut consumer = pool.into_consumer();
            let mut sink = CollectSink::new();
            consumer.drain(&mut sink);

            // Only last 8 items should remain in ring
            let items = sink.into_items();
            assert_eq!(items.len(), 8);
        }

        #[test]
        fn worker_pool_num_rings() {
            let pool = WorkerPool::<u64, 64>::new(7);
            assert_eq!(pool.num_rings(), 7);
        }

        #[test]
        fn worker_pool_empty() {
            let pool = WorkerPool::<u64, 64>::new(4);
            let consumer = pool.into_consumer();
            assert!(consumer.is_empty());
            assert_eq!(consumer.num_producers(), 4);
        }

        #[test]
        fn worker_pool_multiple_run_calls() {
            let pool = WorkerPool::<u64, 128>::new(2);

            pool.run(10);
            pool.run(10);

            let mut consumer = pool.into_consumer();
            let mut sink = CollectSink::new();
            consumer.drain(&mut sink);

            // Should have 40 items total (2 rings × 20 items each)
            let items = sink.into_items();
            assert_eq!(items.len(), 40);
        }
    }
}
