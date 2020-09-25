//! prelude that can should be used for eval implementations.

pub(crate) use crate::ir::eval::{ConstAs, Eval, EvalBreak, EvalOutcome, Matches};
pub(crate) use crate::ir::ir;
pub(crate) use crate::ir::IrInterpreter;
pub(crate) use crate::ir::IrValue;
pub(crate) use crate::query::Used;
pub(crate) use crate::{IrError, Spanned};
pub(crate) use runestick::{Shared, Span};
pub(crate) use std::convert::TryFrom;
