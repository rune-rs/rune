//! Types used for defining native modules.
//!
//! A native module is one that provides rune with functions and types through
//! native Rust-based code.

mod function_meta;
mod function_raw_traits;
mod function_traits;
pub(crate) mod module;

use core::fmt;
use core::marker::PhantomData;

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::{meta, ContextError, Docs, IntoComponent, Item, ItemBuf};
use crate::runtime::{
    AttributeMacroHandler, ConstValue, FullTypeOf, FunctionHandler, MacroHandler, StaticType,
    TypeCheck, TypeInfo, TypeOf,
};
use crate::Hash;

pub(crate) use self::function_meta::{AssociatedFunctionName, ToFieldFunction, ToInstance};

#[doc(hidden)]
pub use self::function_meta::{FunctionMetaData, FunctionMetaKind, MacroMetaData, MacroMetaKind};
pub use self::function_traits::{Async, Function, FunctionKind, InstanceFunction, Plain};
#[doc(hidden)]
pub use self::module::Module;

/// Trait to handle the installation of auxilliary functions for a type
/// installed into a module.
pub trait InstallWith {
    /// Hook to install more things into the module.
    fn install_with(_: &mut Module) -> Result<(), ContextError> {
        Ok(())
    }
}

/// Specialized information on `Option` types.
pub(crate) struct UnitType {
    /// Item of the unit type.
    pub(crate) name: Box<str>,
    /// Documentation on the unit type.
    #[cfg(feature = "doc")]
    pub(crate) docs: Docs,
}

/// Specialized information on `GeneratorState` types.
pub(crate) struct InternalEnum {
    /// The name of the internal enum.
    pub(crate) name: &'static str,
    /// The result type.
    pub(crate) base_type: ItemBuf,
    /// The static type of the enum.
    pub(crate) static_type: &'static StaticType,
    /// Internal variants.
    pub(crate) variants: Vec<Variant>,
    /// Documentation for internal enum.
    pub(crate) docs: Docs,
}

impl InternalEnum {
    /// Construct a new handler for an internal enum.
    fn new<N>(name: &'static str, base_type: N, static_type: &'static StaticType) -> Self
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        InternalEnum {
            name,
            base_type: ItemBuf::with_item(base_type),
            static_type,
            variants: Vec::new(),
            docs: Docs::EMPTY,
        }
    }

    /// Register a new variant.
    fn variant<C, M>(
        &mut self,
        name: &'static str,
        type_check: TypeCheck,
        constructor: C,
    ) -> ItemMut<'_>
    where
        C: Function<M, Plain>,
    {
        let constructor: Arc<FunctionHandler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));

        self.variants.push(Variant {
            name,
            type_check: Some(type_check),
            fields: Some(Fields::Unnamed(C::args())),
            constructor: Some(constructor),
            docs: Docs::EMPTY,
        });

        let v = self.variants.last_mut().unwrap();
        ItemMut { docs: &mut v.docs }
    }
}

/// Data for an opaque type. If `spec` is set, indicates things which are known
/// about that type.
pub(crate) struct ModuleType {
    /// The name of the installed type which will be the final component in the
    /// item it will constitute.
    pub(crate) item: ItemBuf,
    /// Type hash.
    pub(crate) hash: Hash,
    /// Type parameters for this item.
    pub(crate) type_parameters: Hash,
    /// Type information for the installed type.
    pub(crate) type_info: TypeInfo,
    /// The specification for the type.
    pub(crate) spec: Option<TypeSpecification>,
    /// Documentation for the type.
    pub(crate) docs: Docs,
}

/// The kind of the variant.
#[derive(Debug)]
pub(crate) enum Fields {
    /// Sequence of named fields.
    Named(&'static [&'static str]),
    /// Sequence of unnamed fields.
    Unnamed(usize),
    /// Empty.
    Empty,
}

/// Metadata about a variant.
pub struct Variant {
    /// The name of the variant.
    pub(crate) name: &'static str,
    /// Type check for the variant.
    pub(crate) type_check: Option<TypeCheck>,
    /// Variant metadata.
    pub(crate) fields: Option<Fields>,
    /// Handler to use if this variant can be constructed through a regular function call.
    pub(crate) constructor: Option<Arc<FunctionHandler>>,
    /// Variant documentation.
    pub(crate) docs: Docs,
}

impl Variant {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            type_check: None,
            fields: None,
            constructor: None,
            docs: Docs::EMPTY,
        }
    }
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Variant")
            .field("fields", &self.fields)
            .field("constructor", &self.constructor.is_some())
            .field("docs", &self.docs)
            .finish()
    }
}

/// The type specification for a native enum.
pub(crate) struct Enum {
    /// The variants.
    pub(crate) variants: Vec<Variant>,
}

/// A type specification.
pub(crate) enum TypeSpecification {
    Struct(Fields),
    Enum(Enum),
}

/// A key that identifies an associated function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) struct AssociatedKey {
    /// The type the associated function belongs to.
    pub(crate) type_hash: Hash,
    /// The kind of the associated function.
    pub(crate) kind: meta::AssociatedKind,
    /// The type parameters of the associated function.
    pub(crate) parameters: Hash,
}

#[derive(Clone)]
pub(crate) struct ModuleFunction {
    pub(crate) item: ItemBuf,
    pub(crate) handler: Arc<FunctionHandler>,
    #[cfg(feature = "doc")]
    pub(crate) is_async: bool,
    #[cfg(feature = "doc")]
    pub(crate) args: Option<usize>,
    #[cfg(feature = "doc")]
    pub(crate) return_type: Option<FullTypeOf>,
    #[cfg(feature = "doc")]
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
    pub(crate) docs: Docs,
}

#[derive(Clone)]
pub(crate) struct ModuleAssociated {
    pub(crate) container: FullTypeOf,
    pub(crate) container_type_info: TypeInfo,
    pub(crate) name: AssociatedFunctionName,
    pub(crate) handler: Arc<FunctionHandler>,
    #[cfg(feature = "doc")]
    pub(crate) is_async: bool,
    #[cfg(feature = "doc")]
    pub(crate) args: Option<usize>,
    #[cfg(feature = "doc")]
    pub(crate) return_type: Option<FullTypeOf>,
    #[cfg(feature = "doc")]
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
    pub(crate) docs: Docs,
}

/// Handle to a macro inserted into a module.
pub(crate) struct ModuleMacro {
    pub(crate) item: ItemBuf,
    pub(crate) handler: Arc<MacroHandler>,
    pub(crate) docs: Docs,
}

/// Handle to an attribute macro inserted into a module.
pub(crate) struct ModuleAttributeMacro {
    pub(crate) item: ItemBuf,
    pub(crate) handler: Arc<AttributeMacroHandler>,
    pub(crate) docs: Docs,
}

/// A constant registered in a module.
pub(crate) struct ModuleConstant {
    pub(crate) item: ItemBuf,
    pub(crate) value: ConstValue,
    pub(crate) docs: Docs,
}

/// Handle to a an item inserted into a module which allows for mutation of item
/// metadata.
///
/// This is returned by methods which insert meta items, such as:
/// * [`Module::raw_fn`].
/// * [`Module::function`].
/// * [`Module::associated_function`].
///
/// While this is also returned by `*_meta` inserting functions, it is instead
/// recommended that you make use of the appropriate macro to capture doc
/// comments instead:
/// * [`Module::macro_meta`].
/// * [`Module::function_meta`].
pub struct ItemMut<'a> {
    docs: &'a mut Docs,
}

impl ItemMut<'_> {
    /// Set documentation for an inserted item.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs);
        self
    }
}

impl fmt::Debug for ItemMut<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemMut").finish_non_exhaustive()
    }
}

/// Handle to a a variant inserted into a module which allows for mutation of
/// its metadata.
pub struct VariantMut<'a, T>
where
    T: TypeOf,
{
    pub(crate) index: usize,
    pub(crate) docs: &'a mut Docs,
    pub(crate) fields: &'a mut Option<Fields>,
    pub(crate) constructor: &'a mut Option<Arc<FunctionHandler>>,
    pub(crate) _marker: PhantomData<&'a mut T>,
}

impl<T> VariantMut<'_, T>
where
    T: TypeOf,
{
    /// Set documentation for an inserted type.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs);
        self
    }

    /// Set static documentation.
    ///
    /// This completely replaces any existing documentation.
    pub fn static_docs(self, docs: &'static [&'static str]) -> Self {
        self.docs.set_docs(docs);
        self
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
    pub fn constructor<F, M>(self, constructor: F) -> Result<Self, ContextError>
    where
        F: Function<M, Plain, Return = T>,
    {
        if self.constructor.is_some() {
            return Err(ContextError::VariantConstructorConflict {
                type_info: T::type_info(),
                index: self.index,
            });
        }

        *self.constructor = Some(Arc::new(move |stack, args| {
            constructor.fn_call(stack, args)
        }));

        Ok(self)
    }

    fn make(self, fields: Fields) -> Result<Self, ContextError> {
        let old = self.fields.replace(fields);

        if old.is_some() {
            return Err(ContextError::ConflictingVariantMeta {
                index: self.index,
                type_info: T::type_info(),
            });
        }

        Ok(self)
    }
}

/// Access enum metadata mutably.
pub struct EnumMut<'a, T>
where
    T: TypeOf,
{
    docs: &'a mut Docs,
    enum_: &'a mut Enum,
    _marker: PhantomData<&'a mut T>,
}

impl<T> EnumMut<'_, T>
where
    T: TypeOf,
{
    /// Set documentation for an inserted type.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs);
        self
    }

    /// Set static documentation.
    ///
    /// This completely replaces any existing documentation.
    pub fn static_docs(self, docs: &'static [&'static str]) -> Self {
        self.docs.set_docs(docs);
        self
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

/// Access internal enum metadata mutably.
pub struct InternalEnumMut<'a, T>
where
    T: TypeOf,
{
    enum_: &'a mut InternalEnum,
    _marker: PhantomData<&'a mut T>,
}

impl<T> InternalEnumMut<'_, T>
where
    T: TypeOf,
{
    /// Set documentation for an inserted internal enum.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.enum_.docs.set_docs(docs);
        self
    }

    /// Set static documentation for an inserted internal enum.
    ///
    /// This completely replaces any existing documentation.
    pub fn static_docs(self, docs: &'static [&'static str]) -> Self {
        self.enum_.docs.set_docs(docs);
        self
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

/// Handle to a a type inserted into a module which allows for mutation of its
/// metadata.
///
/// This is returned by the following methods:
/// * [`Module::ty`] - after a type has been inserted.
/// * [`Module::type_meta`] - to modify type metadata for an already inserted
///   type.
pub struct TypeMut<'a, T>
where
    T: TypeOf,
{
    docs: &'a mut Docs,
    spec: &'a mut Option<TypeSpecification>,
    item: &'a Item,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> TypeMut<'a, T>
where
    T: TypeOf,
{
    /// Set documentation for an inserted type.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs);
        self
    }

    /// Set static documentation.
    ///
    /// This completely replaces any existing documentation.
    pub fn static_docs(self, docs: &'static [&'static str]) -> Self {
        self.docs.set_docs(docs);
        self
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
            variants: variants.iter().copied().map(Variant::new).collect(),
        }));

        if old.is_some() {
            return Err(ContextError::ConflictingTypeMeta {
                item: self.item.to_owned(),
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

    fn make_struct(self, fields: Fields) -> Result<Self, ContextError> {
        let old = self.spec.replace(TypeSpecification::Struct(fields));

        if old.is_some() {
            return Err(ContextError::ConflictingTypeMeta {
                item: self.item.to_owned(),
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
