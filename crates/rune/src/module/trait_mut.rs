use core::fmt;

use rust_alloc::sync::Arc;

#[cfg(feature = "doc")]
use crate::alloc::Box;
use crate::alloc::Vec;
use crate::compile::context::{TraitContext, TraitHandler};
use crate::compile::{ContextError, Docs};
use crate::function_meta::ToInstance;

use super::{DocFunction, ItemFnMut, ModuleItemCommon, TraitFunction};

/// Handle to a a trait inserted into a module which allows for mutation of its
/// metadata.
pub struct TraitMut<'a> {
    pub(super) docs: &'a mut Docs,
    #[cfg(feature = "doc")]
    pub(super) deprecated: &'a mut Option<Box<str>>,
    pub(super) handler: &'a mut Option<Arc<TraitHandler>>,
    pub(super) functions: &'a mut Vec<TraitFunction>,
}

impl TraitMut<'_> {
    /// Set documentation for an inserted trait.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(&mut self, docs: I) -> Result<&mut Self, ContextError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs)?;
        Ok(self)
    }

    /// Set static documentation.
    ///
    /// This completely replaces any existing documentation.
    pub fn static_docs(
        &mut self,
        docs: &'static [&'static str],
    ) -> Result<&mut Self, ContextError> {
        self.docs.set_docs(docs)?;
        Ok(self)
    }

    /// Mark the given trait as deprecated.
    pub fn deprecated<S>(
        &mut self,
        #[cfg_attr(not(feature = "doc"), allow(unused))] deprecated: S,
    ) -> Result<&mut Self, ContextError>
    where
        S: AsRef<str>,
    {
        #[cfg(feature = "doc")]
        {
            *self.deprecated = Some(deprecated.as_ref().try_into()?);
        }

        Ok(self)
    }

    /// Define a trait handler.
    pub fn handler<F>(&mut self, handler: F) -> Result<&mut Self, ContextError>
    where
        F: 'static + Fn(&mut TraitContext<'_>) -> Result<(), ContextError> + Send + Sync,
    {
        *self.handler = Some(Arc::new(handler));
        Ok(self)
    }

    /// Define a function on the trait.
    pub fn function(&mut self, name: impl ToInstance) -> Result<ItemFnMut<'_>, ContextError> {
        let name = name.to_instance()?;

        self.functions.try_push(TraitFunction {
            name,
            common: ModuleItemCommon::default(),
            doc: DocFunction::default(),
        })?;

        let f = self.functions.last_mut().unwrap();

        Ok(ItemFnMut {
            docs: &mut f.common.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut f.common.deprecated,
            #[cfg(feature = "doc")]
            is_async: &mut f.doc.is_async,
            #[cfg(feature = "doc")]
            args: &mut f.doc.args,
            #[cfg(feature = "doc")]
            return_type: &mut f.doc.return_type,
            #[cfg(feature = "doc")]
            argument_types: &mut f.doc.argument_types,
        })
    }
}

impl fmt::Debug for TraitMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TraitMut").finish_non_exhaustive()
    }
}
