use crate::compile::Result;

use super::{Expectation, Kind, Span};

/// Helper trait to coerce a kind into ast.
pub(crate) trait ToAst
where
    Self: Sized,
{
    /// Coerce a value into ast.
    fn to_ast(span: Span, kind: Kind) -> Result<Self>;

    /// Test if the given ast matches.
    fn matches(kind: &Kind) -> bool;

    /// Get the expectation for this type.
    fn into_expectation() -> Expectation;
}
