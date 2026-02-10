//! Derive macros for bytecast.

mod from_bytes;
mod to_bytes;

pub use from_bytes::derive_from_bytes;
pub use to_bytes::derive_to_bytes;

/// Extract the discriminant type from `#[repr(uN)]` on an enum.
/// Returns `None` if no repr or a non-integer repr is used (defaults to u8).
pub fn repr_int_type(attrs: &[syn::Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }
        let mut found = None;
        let _ = attr.parse_nested_meta(|meta| {
            let ident = meta.path.get_ident().map(|id| id.to_string());
            if let Some("u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64") =
                ident.as_deref()
            {
                found = Some(meta.path.get_ident().unwrap().clone());
            }
            Ok(())
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Return the max number of variants a discriminant type can hold.
pub fn disc_capacity(disc_type: &str) -> usize {
    match disc_type {
        "u8" | "i8" => 256,
        "u16" | "i16" => 65536,
        _ => usize::MAX, // u32/u64/i32/i64 — effectively unlimited
    }
}

/// Check if a field has `#[bytecast(skip)]`.
pub fn has_skip_attr(field: &syn::Field) -> bool {
    field.attrs.iter().any(|attr| {
        if !attr.path().is_ident("bytecast") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                found = true;
            }
            Ok(())
        });
        found
    })
}
