use core::fmt;
use core::marker::PhantomData;

use rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::compile::{ContextError, Docs};
use crate::function::{Function, Plain};
use crate::runtime::{FunctionHandler, TypeOf};
use crate::Item;

use super::{Enum, EnumMut, Fields, TypeSpecification, Variant};

/// Handle to a a type inserted into a module which allows for mutation of its
/// metadata.
///
/// This is returned by the following methods:
/// * [`Module::ty`] - after a type has been inserted.
/// * [`Module::type_meta`] - to modify type metadata for an already inserted
///   type.
///
/// [`Module::ty`]: super::Module::ty
/// [`Module::type_meta`]: super::Module::type_meta
pub struct TypeMut<'a, T>
where
    T: ?Sized + TypeOf,
{
    pub(super) docs: &'a mut Docs,
    #[cfg(feature = "doc")]
    pub(super) deprecated: &'a mut Option<Box<str>>,
    pub(super) spec: &'a mut Option<TypeSpecification>,
    pub(super) constructor: &'a mut Option<Arc<FunctionHandler>>,
    pub(super) item: &'a Item,
    pub(super) _marker: PhantomData<T>,
}

impl<'a, T> TypeMut<'a, T>
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
            *self.deprecated = Some(deprecated.as_ref().try_into()?);
        }

        Ok(self)
    }

    /// Mark the current type as a struct with named fields.
    pub fn make_named_struct(self, fields: &'static [&'static str]) -> Result<Self, ContextError> {
        self.make_struct(Fields::Named(fields))
    }

    /// Mark the current type as a struct with unnamed fields.
    pub fn make_unnamed_struct(self, fields: usize) -> Result<Self, ContextError> {
        self.make_struct(Fields::Unnamed(fields))
    }

    /// Mark the current type as an empty struct.
    pub fn make_empty_struct(self) -> Result<Self, ContextError> {
        self.make_struct(Fields::Empty)
    }

    /// Mark the current type as an enum.
    pub fn make_enum(
        self,
        variants: &'static [&'static str],
    ) -> Result<EnumMut<'a, T>, ContextError> {
        let old = self.spec.replace(TypeSpecification::Enum(Enum {
            variants: variants.iter().copied().map(Variant::new).try_collect()?,
        }));

        if old.is_some() {
            return Err(ContextError::ConflictingTypeMeta {
                item: self.item.try_to_owned()?,
                type_info: T::type_info(),
            });
        }

        let Some(TypeSpecification::Enum(enum_)) = self.spec.as_mut() else {
            panic!("Not an enum");
        };

        Ok(EnumMut {
            docs: self.docs,
            enum_,
            _marker: PhantomData,
        })
    }

    /// Register a constructor method for the current type.
    pub fn constructor<F, A>(self, constructor: F) -> Result<Self, ContextError>
    where
        F: Function<A, Plain, Return = T>,
    {
        if self.constructor.is_some() {
            return Err(ContextError::ConstructorConflict {
                type_info: T::type_info(),
            });
        }

        *self.constructor = Some(Arc::new(move |stack, addr, args, output| {
            constructor.fn_call(stack, addr, args, output)
        }));

        Ok(self)
    }

    fn make_struct(self, fields: Fields) -> Result<Self, ContextError> {
        let old = self.spec.replace(TypeSpecification::Struct(fields));

        if old.is_some() {
            return Err(ContextError::ConflictingTypeMeta {
                item: self.item.try_to_owned()?,
                type_info: T::type_info(),
            });
        }

        Ok(self)
    }
}

impl<T> fmt::Debug for TypeMut<'_, T>
where
    T: TypeOf,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeMut").finish_non_exhaustive()
    }
}
