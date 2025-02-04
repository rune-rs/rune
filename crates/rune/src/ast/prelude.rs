//! Prelude for ast elements.

pub(crate) use crate as rune;
pub(crate) use crate::alloc;
pub(crate) use crate::alloc::prelude::*;
pub(crate) use crate::ast;
pub(crate) use crate::ast::{OptionSpanned, Span, Spanned, ToAst};
pub(crate) use crate::compile::{self, ErrorKind, ItemId};
pub(crate) use crate::macros::{MacroContext, SyntheticKind, ToTokens, TokenStream};
pub(crate) use crate::parse::{
    Expectation, IntoExpectation, NonZeroId, Parse, Parser, Peek, Peeker, Resolve, ResolveContext,
};

pub(crate) type Result<T, E = compile::Error> = core::result::Result<T, E>;

#[cfg(all(test, not(miri)))]
pub(crate) use crate::ast::testing::{rt, rt_with};
