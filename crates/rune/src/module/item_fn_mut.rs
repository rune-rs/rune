use core::fmt;

use crate::alloc::Box;
use crate::compile::meta;
use crate::compile::{ContextError, Docs};
use crate::function_meta::FunctionArgs;
use crate::runtime::MaybeTypeOf;

/// Handle to a an item inserted into a module which allows for mutation of item
/// metadata.
///
/// This is returned by methods which insert meta items, such as:
/// * [`Module::raw_function`].
/// * [`Module::function`].
/// * [`Module::associated_function`].
///
/// While this is also returned by `*_meta` inserting functions, it is instead
/// recommended that you make use of the appropriate macro to capture doc
/// comments instead:
/// * [`Module::macro_meta`].
/// * [`Module::function_meta`].
///
/// [`Module::raw_function`]: super::Module::raw_function
/// [`Module::function`]: super::Module::function
/// [`Module::associated_function`]: super::Module::associated_function
/// [`Module::macro_meta`]: super::Module::macro_meta
/// [`Module::function_meta`]: super::Module::function_meta
pub struct ItemFnMut<'a> {
    pub(super) docs: &'a mut Docs,
    #[cfg(feature = "doc")]
    pub(super) deprecated: &'a mut Option<Box<str>>,
    pub(super) is_async: &'a mut bool,
    pub(super) args: &'a mut Option<usize>,
    pub(super) argument_types: &'a mut Box<[meta::TypeHash]>,
    pub(super) return_type: &'a mut meta::TypeHash,
}

impl ItemFnMut<'_> {
    /// Set documentation for an inserted item.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs(self, docs: impl IntoIterator<Item: AsRef<str>>) -> Result<Self, ContextError> {
        self.docs.set_docs(docs)?;
        Ok(self)
    }

    /// Mark the given item as an async function.
    pub fn is_async(self, is_async: bool) -> Self {
        *self.is_async = is_async;
        self
    }

    /// Mark the given item as deprecated.
    pub fn deprecated(
        self,
        #[cfg_attr(not(feature = "doc"), allow(unused))] deprecated: impl AsRef<str>,
    ) -> Result<Self, ContextError> {
        #[cfg(feature = "doc")]
        {
            *self.deprecated = Some(deprecated.as_ref().try_into()?);
        }

        Ok(self)
    }

    /// Indicate the number of arguments this function accepts.
    pub fn args(self, args: usize) -> Self {
        *self.args = Some(args);
        self
    }

    /// Set the kind of return type.
    pub fn return_type<T>(self) -> Result<Self, ContextError>
    where
        T: MaybeTypeOf,
    {
        *self.return_type = T::maybe_type_of()?;
        Ok(self)
    }

    /// Set argument types.
    pub fn argument_types<A>(self) -> Result<Self, ContextError>
    where
        A: FunctionArgs,
    {
        *self.argument_types = A::into_box()?;
        *self.args = Some(A::len());
        Ok(self)
    }

    /// Set argument names.
    pub fn argument_names(
        self,
        names: impl IntoIterator<Item: AsRef<str>>,
    ) -> Result<Self, ContextError> {
        self.docs.set_arguments(names)?;
        Ok(self)
    }
}

impl fmt::Debug for ItemFnMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemFnMut").finish_non_exhaustive()
    }
}
