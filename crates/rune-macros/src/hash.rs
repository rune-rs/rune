use core::mem::take;

use rune_core::hash::Hash;
use rune_core::item::{ComponentRef, ItemBuf};
use syn::parse::{Parse, ParseStream};

use crate::context::Context;

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

    /// Build a type item based on the current path.
    pub(crate) fn build_type_item(&self, cx: &Context) -> Result<syn::ExprArray, ()> {
        match crate::item::build_item(&self.path) {
            Ok(type_item) => Ok(type_item),
            Err(error) => {
                cx.error(error);
                Err(())
            }
        }
    }

    /// Construct a type hash from a path.
    pub(crate) fn build_type_hash(&self, cx: &Context) -> Result<Hash, ()> {
        self.build_type_hash_with_inner(cx, None)
    }

    /// Construct a type hash from a path with an extra string component at the end.
    pub(crate) fn build_type_hash_with(&self, cx: &Context, extra: &str) -> Result<Hash, ()> {
        self.build_type_hash_with_inner(cx, Some(extra))
    }

    fn build_type_hash_with_inner(&self, cx: &Context, extra: Option<&str>) -> Result<Hash, ()> {
        // Construct type hash.
        let mut buf = ItemBuf::new();
        let mut first = self.path.leading_colon.is_some();

        for s in &self.path.segments {
            let ident = s.ident.to_string();

            let c = if take(&mut first) {
                ComponentRef::Crate(&ident)
            } else {
                ComponentRef::Str(&ident)
            };

            if let Err(error) = buf.push(c) {
                cx.error(syn::Error::new_spanned(s, error));
                return Err(());
            }

            match &s.arguments {
                syn::PathArguments::None => {}
                syn::PathArguments::AngleBracketed(generics) => {
                    cx.error(syn::Error::new_spanned(
                        generics,
                        "Generic arguments are not supported",
                    ));
                }
                syn::PathArguments::Parenthesized(generics) => {
                    cx.error(syn::Error::new_spanned(
                        generics,
                        "Generic arguments are not supported",
                    ));
                }
            }
        }

        if let Some(extra) = extra {
            if let Err(error) = buf.push(ComponentRef::Str(extra)) {
                cx.error(syn::Error::new_spanned(&self.path, error));
                return Err(());
            }
        }

        let base = Hash::type_hash(&buf);

        let hash = if let Some((_, associated)) = &self.associated {
            let name = associated.to_string();
            Hash::associated_function(base, name.as_str())
        } else {
            base
        };

        Ok(hash)
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
