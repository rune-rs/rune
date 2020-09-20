use crate::internals::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote_spanned;
use quote::{quote, ToTokens};
use syn::spanned::Spanned as _;
use syn::Meta::*;
use syn::NestedMeta::*;

/// Parsed `#[rune(..)]` field attributes.
#[derive(Default)]
pub(crate) struct FieldAttrs {
    pub(crate) iter: bool,
    pub(crate) skip: bool,
    pub(crate) optional: bool,
}

/// Parsed ast derive attributes.
#[derive(Default)]
pub(crate) struct DeriveAttrs {}

pub(crate) struct Context {
    pub(crate) errors: Vec<syn::Error>,
    pub(crate) to_tokens: TokenStream,
    pub(crate) spanned: TokenStream,
    pub(crate) option_spanned: TokenStream,
    pub(crate) span: TokenStream,
    pub(crate) macro_context: TokenStream,
    pub(crate) token_stream: TokenStream,
    pub(crate) parser: TokenStream,
    pub(crate) parse: TokenStream,
    pub(crate) parse_error: TokenStream,
}

impl Context {
    /// Construct a new context.
    pub(crate) fn new() -> Self {
        Self::with_module(&quote!(crate))
    }

    /// Construct a new context.
    pub(crate) fn with_module<M>(module: M) -> Self
    where
        M: Copy + ToTokens,
    {
        Self {
            errors: Vec::new(),
            to_tokens: quote!(#module::ToTokens),
            spanned: quote!(#module::Spanned),
            option_spanned: quote!(#module::OptionSpanned),
            span: quote!(runestick::Span),
            macro_context: quote!(#module::MacroContext),
            token_stream: quote!(#module::TokenStream),
            parser: quote!(#module::Parser),
            parse: quote!(#module::Parse),
            parse_error: quote!(#module::ParseError),
        }
    }

    /// Get a field identifier.
    pub(crate) fn field_ident<'a>(&mut self, field: &'a syn::Field) -> Option<&'a syn::Ident> {
        match &field.ident {
            Some(ident) => Some(ident),
            None => {
                self.errors.push(syn::Error::new_spanned(
                    field,
                    "unnamed fields are not supported",
                ));
                None
            }
        }
    }

    /// Parse the toplevel component of the attribute, which must be `#[parse(..)]`.
    fn get_meta_items(
        &mut self,
        attr: &syn::Attribute,
        symbol: Symbol,
    ) -> Option<Vec<syn::NestedMeta>> {
        if attr.path != symbol {
            return Some(Vec::new());
        }

        match attr.parse_meta() {
            Ok(List(meta)) => Some(meta.nested.into_iter().collect()),
            Ok(other) => {
                self.errors.push(syn::Error::new_spanned(
                    other,
                    format!("expected #[{}(...)]", symbol),
                ));
                None
            }
            Err(error) => {
                self.errors.push(syn::Error::new(Span::call_site(), error));
                None
            }
        }
    }

    /// Parse field attributes.
    pub(crate) fn pase_derive_attributes(
        &mut self,
        input: &[syn::Attribute],
    ) -> Option<DeriveAttrs> {
        let attrs = DeriveAttrs::default();

        for attr in input {
            #[allow(clippy::never_loop)] // I guess this is on purpose?
            for meta in self.get_meta_items(attr, RUNE)? {
                match meta {
                    meta => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(attrs)
    }

    /// Parse `#[rune(..)]` field attributes.
    pub(crate) fn parse_field_attributes(
        &mut self,
        input: &[syn::Attribute],
    ) -> Option<FieldAttrs> {
        let mut attrs = FieldAttrs::default();

        for attr in input {
            #[allow(clippy::never_loop)] // I guess this is on purpose?
            for meta in self.get_meta_items(attr, RUNE)? {
                match meta {
                    // Parse `#[rune(iter)]`.
                    Meta(Path(word)) if word == ITER => {
                        attrs.iter = true;
                    }
                    // Parse `#[rune(skip)]`.
                    Meta(Path(word)) if word == SKIP => {
                        attrs.skip = true;
                    }
                    // Parse `#[rune(optional)]`.
                    Meta(Path(word)) if word == OPTIONAL => {
                        attrs.optional = true;
                    }
                    meta => {
                        self.errors
                            .push(syn::Error::new_spanned(meta, "unsupported attribute"));

                        return None;
                    }
                }
            }
        }

        Some(attrs)
    }

    /// Build an inner spanned decoder from an iterator.
    pub(crate) fn build_spanned_iter<'a>(
        &mut self,
        back: bool,
        mut it: impl Iterator<Item = (Option<TokenStream>, &'a syn::Field)>,
    ) -> Option<(bool, Option<TokenStream>)> {
        let mut quote = None::<TokenStream>;

        loop {
            let (var, field) = match it.next() {
                Some((var, field)) => (var?, field),
                None => {
                    return Some((true, quote));
                }
            };

            let attrs = self.parse_field_attributes(&field.attrs)?;

            let spanned = &self.spanned;

            if attrs.skip {
                continue;
            }

            if attrs.optional {
                let option_spanned = &self.option_spanned;
                let next = quote_spanned! {
                    field.span() => #option_spanned::option_span(#var)
                };

                if quote.is_some() {
                    quote = Some(quote_spanned! {
                        field.span() => #quote.or_else(|| #next)
                    });
                } else {
                    quote = Some(next);
                }

                continue;
            }

            if attrs.iter {
                let next = if back {
                    quote_spanned!(field.span() => next_back)
                } else {
                    quote_spanned!(field.span() => next)
                };

                let spanned = &self.spanned;
                let next = quote_spanned! {
                    field.span() => IntoIterator::into_iter(#var).#next().map(#spanned::span)
                };

                if quote.is_some() {
                    quote = Some(quote_spanned! {
                        field.span() => #quote.or_else(|| #next)
                    });
                } else {
                    quote = Some(next);
                }

                continue;
            }

            if quote.is_some() {
                quote = Some(quote_spanned! {
                    field.span() => #quote.unwrap_or_else(|| #spanned::span(#var))
                });
            } else {
                quote = Some(quote_spanned! {
                    field.span() => #spanned::span(#var)
                });
            }

            return Some((false, quote));
        }
    }
}
