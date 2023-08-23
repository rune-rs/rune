use core::mem::take;

use rune_core::{ComponentRef, Hash, ItemBuf};

/// Construct a type hash from a Rust path.
pub(crate) fn build_type_hash(path: &syn::Path) -> syn::Result<Hash> {
    // Construct type hash.
    let mut buf = ItemBuf::new();
    let mut first = path.leading_colon.is_some();

    for s in &path.segments {
        let ident = s.ident.to_string();

        if take(&mut first) {
            buf.push(ComponentRef::Crate(&ident));
        } else {
            buf.push(ComponentRef::Str(&ident));
        }

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

    Ok(Hash::type_hash(&buf))
}
