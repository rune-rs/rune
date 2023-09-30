//! Prelude for ast elements.

pub(crate) use crate as rune;
pub(crate) use crate::alloc::prelude::*;
pub(crate) use crate::alloc::{self, try_vec, Box, String, Vec};
pub(crate) use crate::ast;
pub(crate) use crate::ast::{OptionSpanned, Span, Spanned};
pub(crate) use crate::compile::{self, ErrorKind};
pub(crate) use crate::macros::{MacroContext, SyntheticKind, ToTokens, TokenStream};
pub(crate) use crate::parse::Opaque;
pub(crate) use crate::parse::{
    Expectation, Id, IntoExpectation, Parse, Parser, Peek, Peeker, Resolve, ResolveContext,
};

pub(crate) type Result<T, E = compile::Error> = core::result::Result<T, E>;
