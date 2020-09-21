use crate::CompileError;
use runestick::ConstValue;

mod expr;
mod expr_binary;
mod expr_lit;
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
