use crate::const_compiler::ConstCompiler;
use crate::{CompileError, Spanned};
use runestick::ConstValue;

mod block;
mod condition;
mod expr;
mod expr_binary;
mod expr_block;
mod expr_if;
mod expr_let;
mod expr_lit;
mod expr_while;
mod lit_template;
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
    fn eval(&mut self, value: T, used: Used) -> Result<Option<ConstValue>, CompileError>;
}

pub(crate) trait ConstAs {
    /// Process constant value as a boolean.
    fn as_bool(self, compiler: &mut ConstCompiler<'_>, used: Used) -> Result<bool, CompileError>;
}

impl<T> ConstAs for T
where
    for<'a> ConstCompiler<'a>: Eval<T>,
    T: Spanned,
{
    fn as_bool(self, compiler: &mut ConstCompiler<'_>, used: Used) -> Result<bool, CompileError> {
        let span = self.span();

        compiler
            .eval(self, used)?
            .ok_or_else(|| CompileError::not_const(span))?
            .into_bool()
            .map_err(|actual| CompileError::const_expected::<_, bool>(span, actual))
    }
}
