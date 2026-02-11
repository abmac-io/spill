//! Checkpoint management using Williams' pebble game algorithm.
//!
//! This module provides `PebbleManager`, a high-level interface for managing
//! checkpoints with O(√T) space complexity and near-optimal I/O operations.

mod builder;
pub mod cold;
mod error;
mod pebble_manager;
mod rebuild;
mod recovery;
mod serializers;
mod stats;
mod traits;
pub mod warm;

pub use builder::PebbleManagerBuilder;
#[cfg(feature = "spill-ring-std")]
pub use cold::ParallelCold;
#[cfg(feature = "spill-ring")]
pub use cold::RingCold;
pub use cold::{ColdTier, DirectStorage, DirectStorageError, RecoverableColdTier};
pub use error::{BuilderError, ErasedPebbleManagerError, PebbleManagerError, Result};
pub use pebble_manager::PebbleManager;
#[cfg(feature = "bytecast")]
pub use serializers::BytecastSerializer;
pub use stats::{PebbleStats, TheoreticalValidation};
pub use traits::{CheckpointSerializer, Checkpointable};
#[cfg(feature = "spill-ring")]
pub use warm::WarmCache;
pub use warm::{NoWarm, WarmTier};
