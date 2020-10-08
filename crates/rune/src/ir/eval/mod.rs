use crate::ir::{IrInterpreter, IrPat, IrValue};
use crate::query::Used;
use crate::{IrError, Spanned};
use runestick::Span;

mod ir;
mod ir_assign;
mod ir_binary;
mod ir_branches;
mod ir_break;
mod ir_call;
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

/// The trait for something that can be evaluated in a constant context.
pub trait IrEval {
    /// The result of the evaluation.
    type Output;

    /// Evaluate the given type.
    fn eval(
        &self,
        interp: &mut IrInterpreter<'_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome>;
}

pub(crate) trait ConstAs {
    /// Process constant value as a boolean.
    fn as_bool(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<bool, IrEvalOutcome>;
}

pub(crate) trait Matches {
    /// Test if the current trait matches the given value.
    fn matches<S>(
        &self,
        compiler: &mut IrInterpreter<'_>,
        value: IrValue,
        used: Used,
        spanned: S,
    ) -> Result<bool, IrEvalOutcome>
    where
        S: Spanned;
}

impl<T> ConstAs for T
where
    T: IrEval<Output = IrValue>,
    T: Spanned,
{
    fn as_bool(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<bool, IrEvalOutcome> {
        let span = self.span();

        let value = self
            .eval(interp, used)?
            .into_bool()
            .map_err(|actual| IrError::expected::<_, bool>(span, &actual))?;

        Ok(value)
    }
}

impl Matches for IrPat {
    fn matches<S>(
        &self,
        interp: &mut IrInterpreter<'_>,
        value: IrValue,
        _used: Used,
        spanned: S,
    ) -> Result<bool, IrEvalOutcome>
    where
        S: Spanned,
    {
        match self {
            IrPat::Ignore => Ok(true),
            IrPat::Binding(name) => {
                interp.scopes.decl(name, value, spanned)?;
                Ok(true)
            }
        }
    }
}

/// The outcome of a constant evaluation.
pub enum IrEvalOutcome {
    /// Encountered expression that is not a valid constant expression.
    NotConst(Span),
    /// A compile error.
    Error(IrError),
    /// Break until the next loop, or the optional label.
    Break(Span, IrEvalBreak),
}

impl IrEvalOutcome {
    /// Encountered ast that is not a constant expression.
    pub(crate) fn not_const<S>(spanned: S) -> Self
    where
        S: Spanned,
    {
        Self::NotConst(spanned.span())
    }
}

impl<T> From<T> for IrEvalOutcome
where
    IrError: From<T>,
{
    fn from(error: T) -> Self {
        Self::Error(IrError::from(error))
    }
}

/// The value of a break.
pub enum IrEvalBreak {
    /// Break the next nested loop.
    Inherent,
    /// The break had a value.
    Value(IrValue),
    /// The break had a label.
    Label(Box<str>),
}
