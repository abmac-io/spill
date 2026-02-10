//! FromBytes derive macro implementation.

use super::{disc_capacity, has_skip_attr, repr_int_type};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derive the `FromBytes` trait for a struct or enum.
pub fn derive_from_bytes(input: TokenStream) -> TokenStream {
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

    let body = match &input.data {
        Data::Struct(data) => {
            let (reads, constructor) = generate_struct(name, &data.fields)?;
            quote! {
                #reads
                Ok((#constructor, offset))
            }
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
            generate_enum(name, data, &disc_ident)?
        }
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "FromBytes derive is not supported for unions.",
            ));
        }
    };

    Ok(quote! {
        impl #impl_generics bytecast::FromBytes for #name #ty_generics #where_clause {
            fn from_bytes(buf: &[u8]) -> Result<(Self, usize), bytecast::BytesError> {
                let mut offset = 0usize;
                #body
            }
        }
    })
}

// =============================================================================
// Struct deserialization
// =============================================================================

fn generate_struct(
    name: &syn::Ident,
    fields: &Fields,
) -> syn::Result<(TokenStream2, TokenStream2)> {
    match fields {
        Fields::Named(named) => {
            let field_reads: Vec<_> = named
                .named
                .iter()
                .map(|f| {
                    let field_name = &f.ident;
                    let field_type = &f.ty;
                    if has_skip_attr(f) {
                        quote! {
                            let #field_name: #field_type = Default::default();
                        }
                    } else {
                        quote! {
                            let (#field_name, consumed) = <#field_type as bytecast::FromBytes>::from_bytes(&buf[offset..])?;
                            offset += consumed;
                        }
                    }
                })
                .collect();

            let field_names: Vec<_> = named.named.iter().map(|f| &f.ident).collect();
            let constructor = quote! { #name { #(#field_names),* } };

            Ok((quote! { #(#field_reads)* }, constructor))
        }
        Fields::Unnamed(unnamed) => {
            let field_reads: Vec<_> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let field_name =
                        syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
                    let field_type = &f.ty;
                    if has_skip_attr(f) {
                        quote! {
                            let #field_name: #field_type = Default::default();
                        }
                    } else {
                        quote! {
                            let (#field_name, consumed) = <#field_type as bytecast::FromBytes>::from_bytes(&buf[offset..])?;
                            offset += consumed;
                        }
                    }
                })
                .collect();

            let field_names: Vec<_> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site())
                })
                .collect();
            let constructor = quote! { #name(#(#field_names),*) };

            Ok((quote! { #(#field_reads)* }, constructor))
        }
        Fields::Unit => Ok((quote! {}, quote! { #name })),
    }
}

// =============================================================================
// Enum deserialization
// =============================================================================

fn generate_enum(
    name: &syn::Ident,
    data: &syn::DataEnum,
    disc_type: &syn::Ident,
) -> syn::Result<TokenStream2> {
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
                        #idx_lit => Ok((#name::#variant_name, offset))
                    }
                }
                Fields::Unnamed(fields) => {
                    let field_reads: Vec<_> = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            let field_name = syn::Ident::new(
                                &format!("field_{}", i),
                                proc_macro2::Span::call_site(),
                            );
                            let field_type = &f.ty;
                            quote! {
                                let (#field_name, consumed) = <#field_type as bytecast::FromBytes>::from_bytes(&buf[offset..])?;
                                offset += consumed;
                            }
                        })
                        .collect();

                    let field_names: Vec<_> = (0..fields.unnamed.len())
                        .map(|i| {
                            syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site())
                        })
                        .collect();

                    quote! {
                        #idx_lit => {
                            #(#field_reads)*
                            Ok((#name::#variant_name(#(#field_names),*), offset))
                        }
                    }
                }
                Fields::Named(fields) => {
                    let field_reads: Vec<_> = fields
                        .named
                        .iter()
                        .map(|f| {
                            let field_name = &f.ident;
                            let field_type = &f.ty;
                            quote! {
                                let (#field_name, consumed) = <#field_type as bytecast::FromBytes>::from_bytes(&buf[offset..])?;
                                offset += consumed;
                            }
                        })
                        .collect();

                    let field_names: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();

                    quote! {
                        #idx_lit => {
                            #(#field_reads)*
                            Ok((#name::#variant_name { #(#field_names),* }, offset))
                        }
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        let (discriminant, consumed) = <#disc_type as bytecast::FromBytes>::from_bytes(&buf[offset..])?;
        offset += consumed;

        match discriminant {
            #(#match_arms,)*
            _ => Err(bytecast::BytesError::InvalidData {
                message: "invalid enum discriminant"
            })
        }
    })
}
