//! A `no_std` ring buffer that spills overflow to a configurable sink,
//! plus foundational byte serialization traits.
//!
//! # Ring Buffer
//!
//! ```
//! use spill_ring::{SpillRing, FnSink};
//!
//! let ring = SpillRing::<i32, 4, _>::with_sink(FnSink(|x| println!("evicted: {}", x)));
//!
//! ring.push(1);
//! ring.push(2);
//! ring.push(3);
//! ring.push(4);
//! ring.push(5); // Evicts 1 to sink
//! ```
//!
//! # Byte Serialization
//!
//! ```
//! use spill_ring::{ToBytes, FromBytes};
//!
//! let value: u32 = 12345;
//! let mut buf = [0u8; 4];
//!
//! let written = value.to_bytes(&mut buf).unwrap();
//! let (decoded, _) = u32::from_bytes(&buf).unwrap();
//! assert_eq!(decoded, 12345);
//! ```

#![no_std]
#![warn(missing_docs)]

pub mod bytes;
mod index;
mod iter;
mod ring;
mod sink;
mod traits;

#[cfg(test)]
mod tests;

// Bytes re-exports
pub use bytes::{BytesError, FromBytes, ToBytes, ViewBytes};

// Ring buffer re-exports
pub use iter::{SpillRingIter, SpillRingIterMut};
pub use ring::SpillRing;
pub use sink::{sink, DropSink, Flush, FnFlushSink, FnSink, Sink};
pub use traits::{RingConsumer, RingProducer, RingTrait};
