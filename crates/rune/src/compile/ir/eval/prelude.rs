//! prelude that can should be used for eval implementations.

pub(crate) use crate::ast::{Span, Spanned};
pub(crate) use crate::compile::ir;
pub(crate) use crate::compile::ir::eval::{IrEval, IrEvalBreak, IrEvalOutcome};
pub(crate) use crate::compile::ir::IrInterpreter;
pub(crate) use crate::compile::ir::{IrError, IrValue};
pub(crate) use crate::query::Used;
pub(crate) use crate::runtime::Shared;
pub(crate) use std::convert::TryFrom;
