use crate::compiling::CompileVisitor;
use crate::parsing::{Resolve, ResolveError};
use crate::query::Query;
use crate::shared::Consts;
use crate::{Diagnostics, Options, Storage, UnitBuilder};
use runestick::{Context, Location, Source, Span};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) mod assemble;
pub(crate) mod branches;
pub(crate) mod scope;

pub(crate) use self::assemble::{Assemble, AssembleFn};
pub(crate) use self::branches::Branches;

#[allow(unused)]
pub(crate) struct Compiler<'a> {
    /// Program being compiled.
    pub(crate) program: &'a mut rune_ssa::Program,
    /// The source id of the source.
    pub(crate) location: Location,
    /// The source we are compiling for.
    pub(crate) source: &'a Arc<Source>,
    /// The current scope stack.
    pub(crate) scope: scope::Stack,
    /// The current macro context.
    pub(crate) storage: &'a Storage,
    /// The context we are compiling for.
    pub(crate) context: &'a Context,
    /// Constants storage.
    pub(crate) consts: &'a Consts,
    /// Query system to compile required items.
    pub(crate) query: &'a Query,
    /// The compilation unit we are compiling for.
    pub(crate) unit: UnitBuilder,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Compilation diagnostics.
    pub(crate) diagnostics: &'a mut Diagnostics,
    /// Compiler visitor.
    pub(crate) visitor: Rc<dyn CompileVisitor>,
}

impl<'a> Compiler<'a> {
    /// Resolve the given value.
    pub(crate) fn resolve<T>(&self, value: &T) -> Result<T::Output, ResolveError>
    where
        T: Resolve<'a>,
    {
        value.resolve(&*self.storage, self.source)
    }
}
