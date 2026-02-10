//! Derive macros for bytecast.

use proc_macro::TokenStream;
mod bytes;

/// Derive `ToBytes`.
#[proc_macro_derive(ToBytes, attributes(bytecast))]
pub fn derive_to_bytes(input: TokenStream) -> TokenStream {
    bytes::derive_to_bytes(input)
}

/// Derive `FromBytes`.
#[proc_macro_derive(FromBytes, attributes(bytecast))]
pub fn derive_from_bytes(input: TokenStream) -> TokenStream {
    bytes::derive_from_bytes(input)
}
