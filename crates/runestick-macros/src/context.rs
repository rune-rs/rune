use crate::internals::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Meta::*;

/// Parsed field attributes.
#[derive(Default)]
pub(crate) struct FieldAttrs {}

pub(crate) struct Context {
    pub(crate) errors: Vec<syn::Error>,
    pub(crate) value: TokenStream,
    pub(crate) vm_error: TokenStream,
    pub(crate) vm_error_kind: TokenStream,
    pub(crate) object: TokenStream,
    pub(crate) tuple: TokenStream,
    pub(crate) from_value: TokenStream,
    pub(crate) to_value: TokenStream,
}

impl Context {
    /// Construct a new context.
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            value: quote!(runestick::Value),
            vm_error: quote!(runestick::VmError),
            vm_error_kind: quote!(runestick::VmErrorKind),
            object: quote!(runestick::Object),
            tuple: quote!(runestick::Tuple),
            from_value: quote!(runestick::FromValue),
            to_value: quote!(runestick::ToValue),
        }
    }

    /// Parse the toplevel component of the attribute, which must be `#[rune(..)]`.
    pub fn get_rune_meta_items(&mut self, attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
        if attr.path != RUNE {
            return Some(Vec::new());
        }

        match attr.parse_meta() {
            Ok(List(meta)) => Some(meta.nested.into_iter().collect()),
            Ok(other) => {
                self.errors
                    .push(syn::Error::new_spanned(other, "expected #[rune(...)]"));
                None
            }
            Err(error) => {
                self.errors.push(syn::Error::new(Span::call_site(), error));
                None
            }
        }
    }

    /// Parse field attributes.
    pub(crate) fn parse_field_attrs(&mut self, attrs: &[syn::Attribute]) -> Option<FieldAttrs> {
        let output = FieldAttrs::default();

        for attr in attrs {
            for meta in self.get_rune_meta_items(attr)? {
                match meta {
                    meta => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(output)
    }
}
