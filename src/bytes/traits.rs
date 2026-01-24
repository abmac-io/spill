//! Byte serialization traits.

use crate::bytes::BytesError;

/// Serialize a value to bytes.
///
/// Caller provides the buffer. Returns bytes written.
///
/// # Example
///
/// ```
/// use spill_ring::ToBytes;
///
/// let value: u32 = 42;
/// let mut buf = [0u8; 4];
/// let written = value.to_bytes(&mut buf).unwrap();
/// assert_eq!(written, 4);
/// ```
pub trait ToBytes {
    /// Maximum serialized size, if known at compile time.
    ///
    /// Enables stack allocation: `let mut buf = [0u8; T::MAX_SIZE.unwrap()];`
    const MAX_SIZE: Option<usize> = None;

    /// Serialize into the provided buffer.
    ///
    /// Returns the number of bytes written.
    fn to_bytes(&self, buf: &mut [u8]) -> Result<usize, BytesError>;

    /// Runtime size calculation (optional, for pre-allocation).
    ///
    /// Default returns `MAX_SIZE` if known.
    #[inline]
    fn byte_len(&self) -> Option<usize> {
        Self::MAX_SIZE
    }
}

/// Deserialize from bytes to an owned value.
///
/// # Example
///
/// ```
/// use spill_ring::FromBytes;
///
/// let bytes = [42u8, 0, 0, 0];
/// let (value, consumed): (u32, usize) = u32::from_bytes(&bytes).unwrap();
/// assert_eq!(value, 42);
/// assert_eq!(consumed, 4);
/// ```
pub trait FromBytes: Sized {
    /// Deserialize from bytes.
    ///
    /// Returns the value and number of bytes consumed.
    fn from_bytes(buf: &[u8]) -> Result<(Self, usize), BytesError>;
}

/// Zero-copy view into serialized bytes.
///
/// For peeking at data without full deserialization.
/// The view borrows from the byte slice.
///
/// Typically used with rkyv archived types or similar zero-copy
/// deserialization frameworks where the serialized bytes can be
/// directly interpreted as the target type.
pub trait ViewBytes<'a>: Sized {
    /// Create a view into the bytes without copying.
    fn view(bytes: &'a [u8]) -> Result<Self, BytesError>;
}
