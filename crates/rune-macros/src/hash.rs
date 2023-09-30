use core::mem::take;

use rune_core::{ComponentRef, Hash, ItemBuf};

/// Construct a type hash from a Rust path.
pub(crate) fn build_type_hash(path: &syn::Path) -> syn::Result<Hash> {
    // Construct type hash.
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

    Ok(Hash::type_hash(&buf))
}
