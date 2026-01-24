//! Byte serialization traits and implementations.
//!
//! This module provides `no_std` compatible traits for serializing and
//! deserializing values to/from byte buffers without requiring allocation.
//!
//! # Traits
//!
//! - [`ToBytes`] - Serialize a value to a caller-provided buffer
//! - [`FromBytes`] - Deserialize a value from bytes
//! - [`ViewBytes`] - Zero-copy view into serialized data
//!
//! # Example
//!
//! ```
//! use spill_ring::{ToBytes, FromBytes};
//!
//! let value: u32 = 12345;
//! let mut buf = [0u8; 4];
//!
//! // Serialize
//! let written = value.to_bytes(&mut buf).unwrap();
//! assert_eq!(written, 4);
//!
//! // Deserialize
//! let (decoded, consumed) = u32::from_bytes(&buf).unwrap();
//! assert_eq!(decoded, 12345);
//! assert_eq!(consumed, 4);
//! ```

mod error;
mod impls;
mod traits;

pub use error::{BytesError, Result};
pub use traits::{FromBytes, ToBytes, ViewBytes};

#[cfg(test)]
mod tests;
