use crate::ir_interpreter::IrInterpreter;
use crate::{CompileError, ParseError, Spanned};
use runestick::{ConstValue, Span};

mod ir;
mod ir_binary;
mod ir_branches;
mod ir_break;
mod ir_decl;
mod ir_loop;
mod ir_object;
mod ir_scope;
mod ir_set;
mod ir_template;
mod ir_tuple;
mod ir_vec;
mod prelude;

/// Indication whether a value is being evaluated because it's being used or not.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Used {
    /// The value is not being used.
    Unused,
    /// The value is being used.
    Used,
}

impl Used {
    /// Test if this used indicates unuse.
    pub(crate) fn is_unused(self) -> bool {
        matches!(self, Self::Unused)
    }
}

pub(crate) trait Eval<T> {
    /// Evaluate the given type.
    fn eval(&mut self, value: T, used: Used) -> Result<ConstValue, EvalOutcome>;
}

pub(crate) trait ConstAs {
    /// Process constant value as a boolean.
    fn as_bool(self, compiler: &mut IrInterpreter<'_>, used: Used) -> Result<bool, EvalOutcome>;
}

impl<T> ConstAs for T
where
    for<'a> IrInterpreter<'a>: Eval<T>,
    T: Spanned,
{
    fn as_bool(self, compiler: &mut IrInterpreter<'_>, used: Used) -> Result<bool, EvalOutcome> {
        let span = self.span();

        let value = compiler
            .eval(self, used)?
            .into_bool()
            .map_err(|actual| CompileError::const_expected::<_, bool>(span, &actual))?;

        Ok(value)
    }
}

pub(crate) enum EvalOutcome {
    /// Encountered ast that is not a constant expression.
    NotConst(Span),
    /// A compile error.
    Error(CompileError),
    /// Break until the next loop, or the optional label.
    Break(Span, EvalBreak),
}

impl EvalOutcome {
    /// Encountered ast that is not a constant expression.
    pub(crate) fn not_const<S>(spanned: S) -> Self
    where
        S: Spanned,
    {
        Self::NotConst(spanned.span())
    }
}

impl From<CompileError> for EvalOutcome {
    fn from(error: CompileError) -> Self {
        Self::Error(error)
    }
}

impl From<ParseError> for EvalOutcome {
    fn from(error: ParseError) -> Self {
        Self::Error(error.into())
    }
}

/// The value of a break.
pub(crate) enum EvalBreak {
    /// Break the next nested loop.
    Inherent,
    /// The break had a value.
    Value(ConstValue),
    /// The break had a label.
    Label(Box<str>),
}
