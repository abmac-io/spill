use alloc::vec::Vec;

use crate::{BytesError, FromBytes, ToBytes, ToBytesExt};

/// Wrapper providing rkyv support for any `ToBytes + FromBytes` type.
///
/// The value is stored as raw bytecast-serialized bytes. Since rkyv
/// natively supports `Vec<u8>` with zero-copy access, the bytecast
/// payload sits directly in the rkyv buffer with no extra copies on read.
///
/// # Encoding
/// ```ignore
/// let wrapped = BytecastRkyv::encode(&my_value)?;
/// let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&wrapped)?;
/// ```
///
/// # Decoding
/// ```ignore
/// let archived = rkyv::access::<ArchivedBytecastRkyv, rkyv::rancor::Error>(&bytes)?;
/// let decoded: BytecastRkyv = rkyv::deserialize::<BytecastRkyv, rkyv::rancor::Error>(archived)?;
/// let my_value: MyType = decoded.decode()?;
/// ```
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct BytecastRkyv {
    bytes: Vec<u8>,
}

impl BytecastRkyv {
    /// Encode a `ToBytes` value into a `BytecastRkyv`.
    pub fn encode<T: ToBytes>(value: &T) -> Result<Self, BytesError> {
        Ok(Self {
            bytes: value.to_vec()?,
        })
    }

    /// Decode the stored bytes into a `FromBytes` value.
    pub fn decode<T: FromBytes>(&self) -> Result<T, BytesError> {
        let (val, _) = T::from_bytes(&self.bytes)?;
        Ok(val)
    }
}
