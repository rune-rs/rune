use core::marker::PhantomData;

use crate::compile::{ContextError, Docs};
use crate::function::{Function, Plain};
use crate::runtime::{FunctionHandler, TypeOf};

use super::{Fields, TypeConstructor};

/// Handle to a a variant inserted into a module which allows for mutation of
/// its metadata.
pub struct VariantMut<'a, T>
where
    T: ?Sized + TypeOf,
{
    pub(crate) name: &'static str,
    pub(crate) docs: &'a mut Docs,
    pub(crate) fields: &'a mut Option<Fields>,
    pub(crate) constructor: &'a mut Option<TypeConstructor>,
    pub(crate) _marker: PhantomData<T>,
}

impl<T> VariantMut<'_, T>
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

    /// Mark the given variant with named fields.
    pub fn make_named(self, fields: &'static [&'static str]) -> Result<Self, ContextError> {
        self.make(Fields::Named(fields))
    }

    /// Mark the given variant with unnamed fields.
    pub fn make_unnamed(self, fields: usize) -> Result<Self, ContextError> {
        self.make(Fields::Unnamed(fields))
    }

    /// Mark the given variant as empty.
    pub fn make_empty(self) -> Result<Self, ContextError> {
        self.make(Fields::Empty)
    }

    /// Register a constructor method for the current variant.
    pub fn constructor<F, A>(self, constructor: F) -> Result<Self, ContextError>
    where
        F: Function<A, Plain, Return = T>,
    {
        if self.constructor.is_some() {
            return Err(ContextError::VariantConstructorConflict {
                type_info: T::type_info(),
                name: self.name,
            });
        }

        let handler = FunctionHandler::new(move |stack, addr, args, output| {
            constructor.call(stack, addr, args, output)
        })?;

        *self.constructor = Some(TypeConstructor {
            handler,
            args: F::ARGS,
        });

        Ok(self)
    }

    fn make(self, fields: Fields) -> Result<Self, ContextError> {
        let old = self.fields.replace(fields);

        if old.is_some() {
            return Err(ContextError::ConflictingVariantMeta {
                type_info: T::type_info(),
                name: self.name,
            });
        }

        Ok(self)
    }
}
