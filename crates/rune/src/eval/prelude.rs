//! prelude that can should be used for eval implementations.

pub(crate) use crate::eval::{ConstAs, Eval, EvalBreak, EvalOutcome, Used};
pub(crate) use crate::ir::*;
pub(crate) use crate::ir_interpreter::IrInterpreter;
pub(crate) use crate::CompileError;
pub(crate) use crate::Spanned;
pub(crate) use runestick::{ConstObject, ConstValue, Span};
pub(crate) use std::convert::TryFrom;
