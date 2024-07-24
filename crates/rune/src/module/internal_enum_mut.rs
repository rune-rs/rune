use core::marker::PhantomData;

use crate::compile::ContextError;
use crate::runtime::TypeOf;

use super::{InternalEnum, ModuleItemCommon, VariantMut};

/// Access internal enum metadata mutably.
pub struct InternalEnumMut<'a, T>
where
    T: ?Sized + TypeOf,
{
    pub(super) enum_: &'a mut InternalEnum,
    pub(super) common: &'a mut ModuleItemCommon,
    pub(super) _marker: PhantomData<T>,
}

impl<T> InternalEnumMut<'_, T>
where
    T: ?Sized + TypeOf,
{
    /// Set documentation for an inserted internal enum.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Result<Self, ContextError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.common.docs.set_docs(docs)?;
        Ok(self)
    }

    /// Set static documentation for an inserted internal enum.
    ///
    /// This completely replaces any existing documentation.
    pub fn static_docs(self, docs: &'static [&'static str]) -> Result<Self, ContextError> {
        self.common.docs.set_docs(docs)?;
        Ok(self)
    }

    /// Mark the given type as deprecated.
    pub fn deprecated<S>(
        self,
        #[cfg_attr(not(feature = "doc"), allow(unused))] deprecated: S,
    ) -> Result<Self, ContextError>
    where
        S: AsRef<str>,
    {
        #[cfg(feature = "doc")]
        {
            self.common.deprecated = Some(deprecated.as_ref().try_into()?);
        }

        Ok(self)
    }

    /// Get the given variant mutably.
    pub fn variant_mut(&mut self, index: usize) -> Result<VariantMut<'_, T>, ContextError> {
        let Some(variant) = self.enum_.variants.get_mut(index) else {
            return Err(ContextError::MissingVariant {
                index,
                type_info: T::type_info(),
            });
        };

        Ok(VariantMut {
            index,
            docs: &mut variant.docs,
            fields: &mut variant.fields,
            constructor: &mut variant.constructor,
            _marker: PhantomData,
        })
    }
}
