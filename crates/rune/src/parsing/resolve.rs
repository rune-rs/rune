use crate::macros::Storage;
use crate::parsing::ParseError;
use runestick::Source;

/// A type that can be resolved to an internal value based on a source.
pub trait Resolve<'a>: ResolveOwned {
    /// The output type being resolved into.
    type Output: 'a;

    /// Resolve the value from parsed AST.
    fn resolve(&self, storage: &Storage, source: &'a Source) -> Result<Self::Output, ParseError>;
}

/// Trait for resolving a token into an owned value.
pub trait ResolveOwned {
    /// The output type being resolved into.
    type Owned;

    /// Resolve into an owned value.
    fn resolve_owned(&self, storage: &Storage, source: &Source) -> Result<Self::Owned, ParseError>;
}
