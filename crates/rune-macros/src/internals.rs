use proc_macro2::Span;
use quote::{ToTokens, TokenStreamExt as _};
use std::fmt;

#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

pub const RUNE: Symbol = Symbol("rune");
pub const SKIP: Symbol = Symbol("skip");
pub const ITER: Symbol = Symbol("iter");
pub const OPTIONAL: Symbol = Symbol("optional");

impl PartialEq<Symbol> for syn::Ident {
    fn eq(&self, word: &Symbol) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Symbol> for &'a syn::Ident {
    fn eq(&self, word: &Symbol) -> bool {
        *self == word.0
    }
}

impl PartialEq<Symbol> for syn::Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl<'a> PartialEq<Symbol> for &'a syn::Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl ToTokens for Symbol {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append(syn::Ident::new(self.0, Span::call_site()));
    }
}
