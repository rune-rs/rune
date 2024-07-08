use core::mem::take;

use proc_macro2::Span;
use rune_core::{ComponentRef, ItemBuf};

/// Construct a static item from a path.
pub(crate) fn build_item(path: &syn::Path) -> syn::Result<syn::ExprArray> {
    let mut buf = ItemBuf::new();
    let mut first = path.leading_colon.is_some();

    for s in &path.segments {
        let ident = s.ident.to_string();

        let c = if take(&mut first) {
            ComponentRef::Crate(&ident)
        } else {
            ComponentRef::Str(&ident)
        };

        buf.push(c)
            .map_err(|error| syn::Error::new_spanned(s, error))?;

        match &s.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(generics) => {
                return Err(syn::Error::new_spanned(
                    generics,
                    "Generic arguments are not supported",
                ));
            }
            syn::PathArguments::Parenthesized(generics) => {
                return Err(syn::Error::new_spanned(
                    generics,
                    "Generic arguments are not supported",
                ));
            }
        }
    }

    let mut elems = syn::punctuated::Punctuated::new();

    for &byte in buf.as_bytes() {
        let byte = syn::LitByte::new(byte, Span::call_site());

        elems.push(syn::Expr::Lit(syn::ExprLit {
            attrs: Vec::new(),
            lit: syn::Lit::Byte(byte),
        }));
    }

    Ok(syn::ExprArray {
        attrs: Vec::new(),
        bracket_token: syn::token::Bracket::default(),
        elems,
    })
}
