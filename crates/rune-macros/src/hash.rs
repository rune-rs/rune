use core::mem::take;

use rune_core::hash::Hash;
use rune_core::item::{ComponentRef, ItemBuf};
use syn::parse::{Parse, ParseStream};

pub(super) struct Arguments {
    path: syn::Path,
    associated: Option<(syn::Token![.], syn::Ident)>,
}

impl Arguments {
    pub(super) fn new(path: syn::Path) -> Self {
        Self {
            path,
            associated: None,
        }
    }
}

impl Parse for Arguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;

        let ident = if let Some(colon) = input.parse::<Option<syn::Token![.]>>()? {
            Some((colon, input.parse()?))
        } else {
            None
        };

        Ok(Self {
            path,
            associated: ident,
        })
    }
}

/// Construct a type hash from a Rust path.
pub(crate) fn build_type_hash(args: &Arguments) -> syn::Result<Hash> {
    // Construct type hash.
    let mut buf = ItemBuf::new();
    let mut first = args.path.leading_colon.is_some();

    for s in &args.path.segments {
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

    let base = Hash::type_hash(&buf);

    let hash = if let Some((_, associated)) = &args.associated {
        let name = associated.to_string();
        Hash::associated_function(base, name.as_str())
    } else {
        base
    };

    Ok(hash)
}
