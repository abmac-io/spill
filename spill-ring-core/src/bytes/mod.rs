//! Byte serialization traits and implementations.

mod error;
mod impls;
mod traits;

pub use error::{BytesError, Result};
pub use traits::{FromBytes, ToBytes, ViewBytes};

#[cfg(test)]
mod tests;
