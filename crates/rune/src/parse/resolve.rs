use crate::compile;
use crate::macros::Storage;
use crate::Sources;

/// A resolve context.
#[derive(Clone, Copy)]
pub struct ResolveContext<'a> {
    /// Sources to use.
    pub(crate) sources: &'a Sources,
    /// Storage to use in resolve context.
    pub(crate) storage: &'a Storage,
}

/// A type that can be resolved to an internal value based on a source.
pub trait Resolve<'a> {
    /// The output type being resolved into.
    type Output: 'a;

    /// Resolve the value from parsed AST.
    fn resolve(&self, cx: ResolveContext<'a>) -> compile::Result<Self::Output>;
}
