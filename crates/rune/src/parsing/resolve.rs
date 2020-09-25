use crate::macros::{MacroContext, Storage};
use crate::parsing::ParseError;
use runestick::Source;

/// A type that can be resolved to an internal value based on a source.
pub trait Resolve<'a> {
    /// The output type being resolved.
    type Output: 'a;

    /// Resolve the value from parsed AST.
    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ParseError>;

    /// Resolve the token from a macro context.
    fn macro_resolve(&self, ctx: &'a MacroContext) -> Result<Self::Output, ParseError> {
        self.resolve(ctx.storage(), ctx.source())
    }
}
