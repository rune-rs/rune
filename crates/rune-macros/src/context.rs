use crate::internals::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Meta::*;
use syn::NestedMeta::*;

/// Parsed `#[ast(..)]` attributes.
#[derive(Default)]
pub(crate) struct AstAttrs {
    pub(crate) skip: bool,
}
/// Parsed `#[parse(..)]` attributes.
#[derive(Default)]
pub(crate) struct ParseAttrs {}

/// Parsed ast derive attributes.
#[derive(Default)]
pub(crate) struct AstDerive {}

pub(crate) struct Context {
    pub(crate) errors: Vec<syn::Error>,
    pub(crate) into_tokens: TokenStream,
    pub(crate) spanned: TokenStream,
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
            into_tokens: quote!(#module::IntoTokens),
            spanned: quote!(#module::Spanned),
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

    /// Parse `#[ast(..)]` field attributes.
    pub(crate) fn parse_ast_fields(&mut self, input: &[syn::Attribute]) -> Option<AstAttrs> {
        let mut attrs = AstAttrs::default();

        for attr in input {
            #[allow(clippy::never_loop)] // I guess this is on purpose?
            for meta in self.get_meta_items(attr, AST)? {
                match meta {
                    // Parse `#[ast(skip)]`.
                    Meta(Path(word)) if word == SKIP => {
                        attrs.skip = true;
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

    /// Parse `#[parse(..)]` field attributes.
    pub(crate) fn parse_parse_fields(&mut self, input: &[syn::Attribute]) -> Option<ParseAttrs> {
        let attrs = ParseAttrs::default();

        for attr in input {
            #[allow(clippy::never_loop)] // I guess this is on purpose?
            for meta in self.get_meta_items(attr, PARSE)? {
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

    /// Parse field attributes.
    pub(crate) fn parse_ast_derive(&mut self, input: &[syn::Attribute]) -> Option<AstDerive> {
        let attrs = AstDerive::default();

        for attr in input {
            #[allow(clippy::never_loop)] // I guess this is on purpose?
            for meta in self.get_meta_items(attr, AST)? {
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
}
