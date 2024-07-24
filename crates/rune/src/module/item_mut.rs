use core::fmt;

#[cfg(feature = "doc")]
use crate::alloc::Box;
use crate::compile::{ContextError, Docs};

/// Handle to a an item inserted into a module which allows for mutation of item
/// metadata.
pub struct ItemMut<'a> {
    pub(super) docs: &'a mut Docs,
    #[cfg(feature = "doc")]
    pub(super) deprecated: &'a mut Option<Box<str>>,
}

impl ItemMut<'_> {
    /// Set documentation for an inserted item.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Result<Self, ContextError>
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
    pub fn static_docs(self, docs: &'static [&'static str]) -> Result<Self, ContextError> {
        self.docs.set_docs(docs)?;
        Ok(self)
    }

    /// Mark the given item as deprecated.
    pub fn deprecated<S>(
        self,
        #[cfg_attr(not(feature = "doc"), allow(unused))] deprecated: S,
    ) -> Result<Self, ContextError>
    where
        S: AsRef<str>,
    {
        #[cfg(feature = "doc")]
        {
            *self.deprecated = Some(deprecated.as_ref().try_into()?);
        }

        Ok(self)
    }
}

impl fmt::Debug for ItemMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemMut").finish_non_exhaustive()
    }
}
