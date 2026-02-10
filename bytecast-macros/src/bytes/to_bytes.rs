//! ToBytes derive macro implementation.

use super::{disc_capacity, has_skip_attr, repr_int_type};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derive the `ToBytes` trait for a struct or enum.
pub fn derive_to_bytes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (body, byte_len_body, max_size_body) = match &input.data {
        Data::Struct(data) => {
            let body = generate_struct(&data.fields)?;
            let byte_len = generate_byte_len_struct(&data.fields);
            let max_size = generate_max_size_struct(&data.fields);
            (body, byte_len, max_size)
        }
        Data::Enum(data) => {
            let disc_type = repr_int_type(&input.attrs);
            let disc_ident = disc_type
                .clone()
                .unwrap_or_else(|| syn::Ident::new("u8", proc_macro2::Span::call_site()));
            let max_variants = disc_capacity(&disc_ident.to_string());
            if data.variants.len() > max_variants {
                return Err(syn::Error::new_spanned(
                    input,
                    format!(
                        "enum has {} variants but discriminant type `{}` supports at most {}. \
                         Add #[repr(u16)], #[repr(u32)], etc. to increase capacity.",
                        data.variants.len(),
                        disc_ident,
                        max_variants,
                    ),
                ));
            }
            let body = generate_enum(data, &disc_ident)?;
            let byte_len = generate_byte_len_enum(data, &disc_ident);
            let max_size = generate_max_size_enum(data, &disc_ident);
            (body, byte_len, max_size)
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "ToBytes derive is not supported for unions.",
            ));
        }
    };

    Ok(quote! {
        impl #impl_generics bytecast::ToBytes for #name #ty_generics #where_clause {
            const MAX_SIZE: Option<usize> = #max_size_body;

            fn to_bytes(&self, buf: &mut [u8]) -> Result<usize, bytecast::BytesError> {
                let mut offset = 0usize;
                #body
                Ok(offset)
            }

            fn byte_len(&self) -> Option<usize> {
                #byte_len_body
            }
        }
    })
}

// Struct serialization

fn generate_struct(fields: &Fields) -> syn::Result<TokenStream2> {
    match fields {
        Fields::Named(named) => {
            let field_writes: Vec<_> = named
                .named
                .iter()
                .filter(|f| !has_skip_attr(f))
                .map(|f| {
                    let name = &f.ident;
                    quote! {
                        let written = bytecast::ToBytes::to_bytes(&self.#name, &mut buf[offset..])?;
                        offset += written;
                    }
                })
                .collect();
            Ok(quote! { #(#field_writes)* })
        }
        Fields::Unnamed(unnamed) => {
            let field_writes: Vec<_> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .filter(|(_, f)| !has_skip_attr(f))
                .map(|(i, _)| {
                    let index = syn::Index::from(i);
                    quote! {
                        let written = bytecast::ToBytes::to_bytes(&self.#index, &mut buf[offset..])?;
                        offset += written;
                    }
                })
                .collect();
            Ok(quote! { #(#field_writes)* })
        }
        Fields::Unit => Ok(quote! {}),
    }
}

fn generate_byte_len_struct(fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Named(named) => {
            let field_lens: Vec<_> = named
                .named
                .iter()
                .filter(|f| !has_skip_attr(f))
                .map(|f| {
                    let name = &f.ident;
                    quote! { bytecast::ToBytes::byte_len(&self.#name)? }
                })
                .collect();

            if field_lens.is_empty() {
                quote! { Some(0) }
            } else {
                quote! { Some(0 #(+ #field_lens)*) }
            }
        }
        Fields::Unnamed(unnamed) => {
            let field_lens: Vec<_> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .filter(|(_, f)| !has_skip_attr(f))
                .map(|(i, _)| {
                    let index = syn::Index::from(i);
                    quote! { bytecast::ToBytes::byte_len(&self.#index)? }
                })
                .collect();

            if field_lens.is_empty() {
                quote! { Some(0) }
            } else {
                quote! { Some(0 #(+ #field_lens)*) }
            }
        }
        Fields::Unit => quote! { Some(0) },
    }
}

fn generate_max_size_struct(fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Named(named) => {
            let non_skipped: Vec<_> = named.named.iter().filter(|f| !has_skip_attr(f)).collect();
            if non_skipped.is_empty() {
                return quote! { Some(0) };
            }
            let field_sizes: Vec<_> = non_skipped
                .iter()
                .map(|f| {
                    let ty = &f.ty;
                    quote! { <#ty as bytecast::ToBytes>::MAX_SIZE }
                })
                .collect();
            quote! {
                {
                    // Use const fn to compute at compile time
                    const fn compute_max_size() -> Option<usize> {
                        let mut total = 0usize;
                        #(
                            match #field_sizes {
                                Some(s) => total += s,
                                None => return None,
                            }
                        )*
                        Some(total)
                    }
                    compute_max_size()
                }
            }
        }
        Fields::Unnamed(unnamed) => {
            let non_skipped: Vec<_> = unnamed
                .unnamed
                .iter()
                .filter(|f| !has_skip_attr(f))
                .collect();
            if non_skipped.is_empty() {
                return quote! { Some(0) };
            }
            let field_sizes: Vec<_> = non_skipped
                .iter()
                .map(|f| {
                    let ty = &f.ty;
                    quote! { <#ty as bytecast::ToBytes>::MAX_SIZE }
                })
                .collect();
            quote! {
                {
                    const fn compute_max_size() -> Option<usize> {
                        let mut total = 0usize;
                        #(
                            match #field_sizes {
                                Some(s) => total += s,
                                None => return None,
                            }
                        )*
                        Some(total)
                    }
                    compute_max_size()
                }
            }
        }
        Fields::Unit => quote! { Some(0) },
    }
}

// Enum serialization

fn generate_enum(data: &syn::DataEnum, disc_type: &syn::Ident) -> syn::Result<TokenStream2> {
    let match_arms: Vec<_> = data
        .variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| {
            let variant_name = &variant.ident;
            let idx_lit = syn::LitInt::new(&idx.to_string(), proc_macro2::Span::call_site());

            match &variant.fields {
                Fields::Unit => {
                    quote! {
                        Self::#variant_name => {
                            let written = bytecast::ToBytes::to_bytes(&(#idx_lit as #disc_type), &mut buf[offset..])?;
                            offset += written;
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let field_names: Vec<_> = (0..fields.unnamed.len())
                        .map(|i| {
                            syn::Ident::new(&format!("f{}", i), proc_macro2::Span::call_site())
                        })
                        .collect();
                    let field_writes: Vec<_> = field_names
                        .iter()
                        .map(|name| {
                            quote! {
                                let written = bytecast::ToBytes::to_bytes(#name, &mut buf[offset..])?;
                                offset += written;
                            }
                        })
                        .collect();

                    quote! {
                        Self::#variant_name(#(#field_names),*) => {
                            let written = bytecast::ToBytes::to_bytes(&(#idx_lit as #disc_type), &mut buf[offset..])?;
                            offset += written;
                            #(#field_writes)*
                        }
                    }
                }
                Fields::Named(fields) => {
                    let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
                    let field_writes: Vec<_> = field_names
                        .iter()
                        .map(|name| {
                            quote! {
                                let written = bytecast::ToBytes::to_bytes(#name, &mut buf[offset..])?;
                                offset += written;
                            }
                        })
                        .collect();

                    quote! {
                        Self::#variant_name { #(#field_names),* } => {
                            let written = bytecast::ToBytes::to_bytes(&(#idx_lit as #disc_type), &mut buf[offset..])?;
                            offset += written;
                            #(#field_writes)*
                        }
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        match self {
            #(#match_arms)*
        }
    })
}

fn generate_byte_len_enum(data: &syn::DataEnum, disc_type: &syn::Ident) -> TokenStream2 {
    let match_arms: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;

            match &variant.fields {
                Fields::Unit => {
                    quote! { Self::#variant_name => Some(core::mem::size_of::<#disc_type>()) }
                }
                Fields::Unnamed(fields) => {
                    let field_names: Vec<_> = (0..fields.unnamed.len())
                        .map(|i| {
                            syn::Ident::new(&format!("f{}", i), proc_macro2::Span::call_site())
                        })
                        .collect();
                    let field_lens: Vec<_> = field_names
                        .iter()
                        .map(|name| quote! { bytecast::ToBytes::byte_len(#name)? })
                        .collect();

                    quote! { Self::#variant_name(#(#field_names),*) => Some(core::mem::size_of::<#disc_type>() #(+ #field_lens)*) }
                }
                Fields::Named(fields) => {
                    let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
                    let field_lens: Vec<_> = field_names
                        .iter()
                        .map(|name| quote! { bytecast::ToBytes::byte_len(#name)? })
                        .collect();

                    quote! { Self::#variant_name { #(#field_names),* } => Some(core::mem::size_of::<#disc_type>() #(+ #field_lens)*) }
                }
            }
        })
        .collect();

    quote! {
        match self {
            #(#match_arms),*
        }
    }
}

fn generate_max_size_enum(data: &syn::DataEnum, disc_type: &syn::Ident) -> TokenStream2 {
    if data.variants.is_empty() {
        return quote! { Some(core::mem::size_of::<#disc_type>()) };
    }

    // Collect max sizes for each variant's fields
    let variant_sizes: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let field_sizes: Vec<_> = match &variant.fields {
                Fields::Named(named) => named
                    .named
                    .iter()
                    .map(|f| {
                        let ty = &f.ty;
                        quote! { <#ty as bytecast::ToBytes>::MAX_SIZE }
                    })
                    .collect(),
                Fields::Unnamed(unnamed) => unnamed
                    .unnamed
                    .iter()
                    .map(|f| {
                        let ty = &f.ty;
                        quote! { <#ty as bytecast::ToBytes>::MAX_SIZE }
                    })
                    .collect(),
                Fields::Unit => vec![],
            };
            field_sizes
        })
        .collect();

    quote! {
        {
            const fn compute_max_size() -> Option<usize> {
                let mut max = 0usize;
                #(
                    {
                        let mut variant_size = 0usize;
                        #(
                            match #variant_sizes {
                                Some(s) => variant_size += s,
                                None => return None,
                            }
                        )*
                        if variant_size > max {
                            max = variant_size;
                        }
                    }
                )*
                Some(core::mem::size_of::<#disc_type>() + max)
            }
            compute_max_size()
        }
    }
}
