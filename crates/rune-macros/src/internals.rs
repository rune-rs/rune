use proc_macro2::Span;
use quote::{ToTokens, TokenStreamExt as _};
use std::fmt;

#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

pub const RUNE: Symbol = Symbol("rune");
pub const ID: Symbol = Symbol("id");
pub const SKIP: Symbol = Symbol("skip");
pub const ITER: Symbol = Symbol("iter");
pub const OPTION: Symbol = Symbol("option");
pub const META: Symbol = Symbol("meta");
pub const SPAN: Symbol = Symbol("span");
pub const PARSE_WITH: Symbol = Symbol("parse_with");
pub const PARSE: Symbol = Symbol("parse");

pub const NAME: Symbol = Symbol("name");
pub const ITEM: Symbol = Symbol("item");
pub const MODULE: Symbol = Symbol("module");
pub const INSTALL_WITH: Symbol = Symbol("install_with");

pub const CONSTRUCTOR: Symbol = Symbol("constructor");
pub const BUILTIN: Symbol = Symbol("builtin");
pub const STATIC_TYPE: Symbol = Symbol("static_type");
pub const FROM_VALUE: Symbol = Symbol("from_value");
pub const FROM_VALUE_PARAMS: Symbol = Symbol("from_value_params");
pub const GET: Symbol = Symbol("get");
pub const SET: Symbol = Symbol("set");
pub const COPY: Symbol = Symbol("copy");

pub const ADD_ASSIGN: Symbol = Symbol("add_assign");
pub const SUB_ASSIGN: Symbol = Symbol("sub_assign");
pub const DIV_ASSIGN: Symbol = Symbol("div_assign");
pub const MUL_ASSIGN: Symbol = Symbol("mul_assign");
pub const BIT_AND_ASSIGN: Symbol = Symbol("bit_and_assign");
pub const BIT_OR_ASSIGN: Symbol = Symbol("bit_or_assign");
pub const BIT_XOR_ASSIGN: Symbol = Symbol("bit_xor_assign");
pub const SHL_ASSIGN: Symbol = Symbol("shl_assign");
pub const SHR_ASSIGN: Symbol = Symbol("shr_assign");
pub const REM_ASSIGN: Symbol = Symbol("rem_assign");

pub const PROTOCOL_GET: Symbol = Symbol("GET");
pub const PROTOCOL_SET: Symbol = Symbol("SET");
pub const PROTOCOL_ADD_ASSIGN: Symbol = Symbol("ADD_ASSIGN");
pub const PROTOCOL_SUB_ASSIGN: Symbol = Symbol("SUB_ASSIGN");
pub const PROTOCOL_DIV_ASSIGN: Symbol = Symbol("DIV_ASSIGN");
pub const PROTOCOL_MUL_ASSIGN: Symbol = Symbol("MUL_ASSIGN");
pub const PROTOCOL_BIT_AND_ASSIGN: Symbol = Symbol("BIT_AND_ASSIGN");
pub const PROTOCOL_BIT_OR_ASSIGN: Symbol = Symbol("BIT_OR_ASSIGN");
pub const PROTOCOL_BIT_XOR_ASSIGN: Symbol = Symbol("BIT_XOR_ASSIGN");
pub const PROTOCOL_SHL_ASSIGN: Symbol = Symbol("SHL_ASSIGN");
pub const PROTOCOL_SHR_ASSIGN: Symbol = Symbol("SHR_ASSIGN");
pub const PROTOCOL_REM_ASSIGN: Symbol = Symbol("REM_ASSIGN");

impl Symbol {
    /// Construct identifier out of symbol.
    #[inline]
    pub(crate) fn to_ident(self, span: Span) -> syn::Ident {
        syn::Ident::new(self.0, span)
    }
}

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
