//! Prelude for ast elements.

pub(crate) use crate::ast;
pub(crate) use crate::ast::utils;
pub(crate) use crate::ast::{OptionSpanned, Span, Spanned};
pub(crate) use crate::macros::{MacroContext, SyntheticKind, ToTokens, TokenStream};
pub(crate) use crate::no_std::prelude::*;
pub(crate) use crate::parse::Opaque;
pub(crate) use crate::parse::{
    Expectation, Id, IntoExpectation, Parse, ParseError, ParseErrorKind, Parser, Peek, Peeker,
    Resolve, ResolveContext, ResolveError, ResolveErrorKind,
};
