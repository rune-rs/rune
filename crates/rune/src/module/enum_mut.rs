use core::marker::PhantomData;

use crate::compile::{ContextError, Docs};
use crate::runtime::TypeOf;

use super::{Enum, VariantMut};

/// Access enum metadata mutably.
pub struct EnumMut<'a, T>
where
    T: ?Sized + TypeOf,
{
    pub(super) docs: &'a mut Docs,
    pub(super) enum_: &'a mut Enum,
    pub(super) _marker: PhantomData<T>,
}

impl<T> EnumMut<'_, T>
where
    T: ?Sized + TypeOf,
{
    /// Set documentation for an inserted type.
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

    /// Get the given variant mutably.
    pub fn variant_mut(&mut self, index: usize) -> Result<VariantMut<'_, T>, ContextError> {
        let Some(variant) = self.enum_.variants.get_mut(index) else {
            return Err(ContextError::MissingVariant {
                index,
                type_info: T::type_info(),
            });
        };

        Ok(VariantMut {
            name: variant.name,
            docs: &mut variant.docs,
            fields: &mut variant.fields,
            constructor: &mut variant.constructor,
            _marker: PhantomData,
        })
    }
}
