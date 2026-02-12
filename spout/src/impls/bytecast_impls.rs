use alloc::vec::Vec;

use bytecast::{ByteCursor, ByteReader, BytesError, FromBytes, ToBytes};

use crate::Spout;

/// Prepends framing headers (producer_id, byte length, payload) before forwarding.
///
/// Each item is serialized via `ToBytes`, then wrapped in a frame:
/// `[producer_id: usize] [payload_len: u32 (4 bytes)] [payload bytes]`
///
/// Note: `usize` is serialized as `u64` on the wire by bytecast, so frames
/// are portable across architectures.
///
/// Compose with `ProducerSpout` for tagged, framed output.
pub struct FramedSpout<S> {
    inner: S,
    producer_id: usize,
    /// Reusable buffer to avoid per-send allocation.
    buf: Vec<u8>,
}

/// Fixed overhead per frame: serialized usize (always 8 bytes via bytecast) + u32 payload length.
const FRAME_HEADER_SIZE: usize = match (<usize as ToBytes>::MAX_SIZE, <u32 as ToBytes>::MAX_SIZE) {
    (Some(a), Some(b)) => a + b,
    _ => unreachable!(),
};

impl<S> FramedSpout<S> {
    /// Create a new framed spout.
    ///
    /// Each item sent will be serialized and wrapped in a frame with the
    /// given `producer_id` before forwarding to the inner spout.
    pub fn new(producer_id: usize, inner: S) -> Self {
        Self {
            inner,
            producer_id,
            buf: Vec::new(),
        }
    }

    /// Get the producer ID.
    pub fn producer_id(&self) -> usize {
        self.producer_id
    }

    /// Get a reference to the inner spout.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Get a mutable reference to the inner spout.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Consume and return the inner spout.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<T: ToBytes, S: Spout<Vec<u8>>> Spout<T> for FramedSpout<S> {
    #[inline]
    fn send(&mut self, item: T) {
        // Determine payload size
        let payload_size = item.byte_len().or(T::MAX_SIZE).unwrap_or(256);

        // Prepare buffer for header + payload
        self.buf.clear();
        self.buf.resize(FRAME_HEADER_SIZE + payload_size, 0);

        // Write payload first to learn actual size
        let payload_written = item
            .to_bytes(&mut self.buf[FRAME_HEADER_SIZE..])
            .expect("FramedSpout: payload serialization failed");

        // Write header: producer_id + payload length
        let mut cursor = ByteCursor::new(&mut self.buf[..FRAME_HEADER_SIZE]);
        cursor
            .write(&self.producer_id)
            .expect("FramedSpout: header serialization failed");
        cursor
            .write(&(payload_written as u32))
            .expect("FramedSpout: header serialization failed");

        // Truncate to actual frame size and swap out — avoids memcpy
        let total = FRAME_HEADER_SIZE + payload_written;
        self.buf.truncate(total);
        let frame = core::mem::replace(
            &mut self.buf,
            Vec::with_capacity(FRAME_HEADER_SIZE + payload_size),
        );
        self.inner.send(frame);
    }

    #[inline]
    fn flush(&mut self) {
        self.inner.flush();
    }
}

/// Decode a frame produced by `FramedSpout`.
///
/// Returns `(producer_id, item)` from the framed bytes.
pub fn decode_frame<T: FromBytes>(frame: &[u8]) -> Result<(usize, T), BytesError> {
    let mut reader = ByteReader::new(frame);
    let producer_id: usize = reader.read()?;
    let _payload_len: u32 = reader.read()?;
    let item: T = reader.read()?;
    Ok((producer_id, item))
}

// --- BatchSpout snapshot serialization ---

use crate::BatchSpout;

impl<T: ToBytes, S> ToBytes for BatchSpout<T, S> {
    const MAX_SIZE: Option<usize> = None;

    fn to_bytes(&self, buf: &mut [u8]) -> Result<usize, bytecast::BytesError> {
        let mut cursor = ByteCursor::new(buf);
        cursor.write(&(self.threshold() as u32))?;
        cursor.write(&self.buffer)?;
        Ok(cursor.position())
    }

    fn byte_len(&self) -> Option<usize> {
        // 4 bytes threshold + buffer byte_len
        Some(4 + self.buffer.byte_len()?)
    }
}
