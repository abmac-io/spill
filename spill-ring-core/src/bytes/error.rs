//! Error types for byte serialization.

/// Error during byte serialization or deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytesError {
    /// Buffer too small for serialization.
    BufferTooSmall {
        /// Bytes needed.
        needed: usize,
        /// Bytes available.
        available: usize,
    },

    /// Invalid data during deserialization.
    InvalidData {
        /// Error description.
        message: &'static str,
    },

    /// Unexpected end of input.
    UnexpectedEof {
        /// Bytes needed.
        needed: usize,
        /// Bytes available.
        available: usize,
    },

    /// Custom error for user implementations.
    Custom {
        /// Error description.
        message: &'static str,
    },
}

impl core::fmt::Display for BytesError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BytesError::BufferTooSmall { needed, available } => {
                write!(
                    f,
                    "buffer too small: needed {} bytes, only {} available",
                    needed, available
                )
            }
            BytesError::InvalidData { message } => {
                write!(f, "invalid data: {}", message)
            }
            BytesError::UnexpectedEof { needed, available } => {
                write!(
                    f,
                    "unexpected end of input: needed {} bytes, only {} available",
                    needed, available
                )
            }
            BytesError::Custom { message } => {
                write!(f, "{}", message)
            }
        }
    }
}

/// Result type for bytes operations.
pub type Result<T> = core::result::Result<T, BytesError>;
