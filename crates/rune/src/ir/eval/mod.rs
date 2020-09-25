use crate::ir::ir::IrPat;
use crate::ir::{IrInterpreter, IrValue};
use crate::query::Used;
use crate::{IrError, QueryError, Spanned};
use runestick::Span;

mod ir;
mod ir_assign;
mod ir_binary;
mod ir_branches;
mod ir_break;
mod ir_condition;
mod ir_decl;
mod ir_loop;
mod ir_object;
mod ir_scope;
mod ir_set;
mod ir_template;
mod ir_tuple;
mod ir_vec;
mod prelude;

pub(crate) trait Eval<T> {
    type Output;

    /// Evaluate the given type.
    fn eval(&mut self, value: T, used: Used) -> Result<Self::Output, EvalOutcome>;
}

pub(crate) trait ConstAs {
    /// Process constant value as a boolean.
    fn as_bool(self, compiler: &mut IrInterpreter<'_>, used: Used) -> Result<bool, EvalOutcome>;
}

pub(crate) trait Matches {
    /// Test if the current trait matches the given value.
    fn matches<S>(
        &self,
        compiler: &mut IrInterpreter<'_>,
        value: IrValue,
        used: Used,
        spanned: S,
    ) -> Result<bool, EvalOutcome>
    where
        S: Spanned;
}

impl<T> ConstAs for T
where
    for<'a> IrInterpreter<'a>: Eval<T, Output = IrValue>,
    T: Spanned,
{
    fn as_bool(self, compiler: &mut IrInterpreter<'_>, used: Used) -> Result<bool, EvalOutcome> {
        let span = self.span();

        let value = compiler
            .eval(self, used)?
            .into_bool()
            .map_err(|actual| IrError::expected::<_, bool>(span, &actual))?;

        Ok(value)
    }
}

impl Matches for IrPat {
    fn matches<S>(
        &self,
        compiler: &mut IrInterpreter<'_>,
        value: IrValue,
        _used: Used,
        spanned: S,
    ) -> Result<bool, EvalOutcome>
    where
        S: Spanned,
    {
        match self {
            IrPat::Ignore => Ok(true),
            IrPat::Binding(name) => {
                compiler.scopes.decl(name, value, spanned)?;
                Ok(true)
            }
        }
    }
}

pub(crate) enum EvalOutcome {
    /// Encountered ast that is not a constant expression.
    NotConst(Span),
    /// A compile error.
    Error(IrError),
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

impl From<IrError> for EvalOutcome {
    fn from(error: IrError) -> Self {
        Self::Error(error)
    }
}

impl From<QueryError> for EvalOutcome {
    fn from(error: QueryError) -> Self {
        Self::Error(error.into())
    }
}

/// The value of a break.
pub(crate) enum EvalBreak {
    /// Break the next nested loop.
    Inherent,
    /// The break had a value.
    Value(IrValue),
    /// The break had a label.
    Label(Box<str>),
}
