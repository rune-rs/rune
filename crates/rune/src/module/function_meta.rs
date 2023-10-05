use core::marker::PhantomData;

use ::rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::borrow::Cow;
use crate::alloc::prelude::*;
#[cfg(feature = "doc")]
use crate::alloc::Vec;
use crate::alloc::{self, try_vec, Box};
use crate::compile::{self, meta, IntoComponent, ItemBuf};
use crate::hash::Hash;
use crate::macros::{MacroContext, TokenStream};
use crate::module::{AssociatedKey, Function, FunctionKind, InstanceFunction};
use crate::runtime::{
    AttributeMacroHandler, FullTypeOf, FunctionHandler, MacroHandler, MaybeTypeOf, Protocol,
    TypeInfo, TypeOf,
};

mod sealed {
    use crate::params::Params;
    use crate::runtime::Protocol;

    pub trait Sealed {}

    impl Sealed for &str {}
    impl Sealed for Protocol {}
    impl<T, const N: usize> Sealed for Params<T, N> {}
}

/// Type used to collect and store function metadata through the
/// `#[rune::function]` macro.
///
/// This is the argument type for
/// [`Module::function_meta`][crate::module::Module::function_meta], and is from
/// a public API perspective completely opaque and might change for any release.
///
/// Calling and making use of `FunctionMeta` manually despite this warning might
/// lead to future breakage.
pub type FunctionMeta = fn() -> alloc::Result<FunctionMetaData>;

/// Type used to collect and store function metadata through the
/// `#[rune::macro_]` macro.
///
/// This is the argument type for
/// [`Module::macro_meta`][crate::module::Module::macro_meta], and is from a
/// public API perspective completely opaque and might change for any release.
///
/// Calling and making use of `MacroMeta` manually despite this warning might
/// lead to future breakage.
pub type MacroMeta = fn() -> alloc::Result<MacroMetaData>;

/// Runtime data for a function.
pub struct FunctionData {
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
}

impl FunctionData {
    pub(crate) fn from_raw(item: ItemBuf, handler: Arc<FunctionHandler>) -> Self {
        Self {
            item,
            handler,
            #[cfg(feature = "doc")]
            is_async: false,
            #[cfg(feature = "doc")]
            args: None,
            #[cfg(feature = "doc")]
            return_type: None,
            #[cfg(feature = "doc")]
            argument_types: Box::default(),
        }
    }

    #[inline]
    pub(crate) fn new<F, A, N, K>(name: N, f: F) -> alloc::Result<Self>
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        N: IntoComponent,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Ok(Self {
            item: ItemBuf::with_item([name])?,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            #[cfg(feature = "doc")]
            is_async: K::is_async(),
            #[cfg(feature = "doc")]
            args: Some(F::args()),
            #[cfg(feature = "doc")]
            return_type: F::Return::maybe_type_of(),
            #[cfg(feature = "doc")]
            argument_types: A::into_box()?,
        })
    }
}

/// Runtime data for a macro.
pub struct FunctionMacroData {
    pub(crate) item: ItemBuf,
    pub(crate) handler: Arc<MacroHandler>,
}

impl FunctionMacroData {
    #[inline]
    pub(crate) fn new<F, N>(name: N, f: F) -> alloc::Result<Self>
    where
        F: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_, '_, '_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Ok(Self {
            item: ItemBuf::with_item(name)?,
            handler: Arc::new(f),
        })
    }
}

/// Runtime data for an attribute macro.
pub struct AttributeMacroData {
    pub(crate) item: ItemBuf,
    pub(crate) handler: Arc<AttributeMacroHandler>,
}

impl AttributeMacroData {
    #[inline]
    pub(crate) fn new<F, N>(name: N, f: F) -> alloc::Result<Self>
    where
        F: 'static
            + Send
            + Sync
            + Fn(
                &mut MacroContext<'_, '_, '_>,
                &TokenStream,
                &TokenStream,
            ) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Ok(Self {
            item: ItemBuf::with_item(name)?,
            handler: Arc::new(f),
        })
    }
}

/// A descriptor for an instance function.
#[derive(Debug, TryClone)]
#[non_exhaustive]
#[doc(hidden)]
pub struct AssociatedName {
    /// The name of the instance function.
    pub kind: meta::AssociatedKind,
    /// Parameters hash.
    pub function_parameters: Hash,
    #[cfg(feature = "doc")]
    pub parameter_types: Vec<Hash>,
}

impl AssociatedName {
    pub(crate) fn index(protocol: Protocol, index: usize) -> Self {
        Self {
            kind: meta::AssociatedKind::IndexFn(protocol, index),
            function_parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: Vec::new(),
        }
    }
}

/// Trait used solely to construct an instance function.
pub trait ToInstance: self::sealed::Sealed {
    /// Get information on the naming of the instance function.
    #[doc(hidden)]
    fn to_instance(self) -> alloc::Result<AssociatedName>;
}

/// Trait used to determine what can be used as an instance function name.
pub trait ToFieldFunction: self::sealed::Sealed {
    #[doc(hidden)]
    fn to_field_function(self, protocol: Protocol) -> alloc::Result<AssociatedName>;
}

impl ToInstance for &'static str {
    #[inline]
    fn to_instance(self) -> alloc::Result<AssociatedName> {
        Ok(AssociatedName {
            kind: meta::AssociatedKind::Instance(Cow::Borrowed(self)),
            function_parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: Vec::new(),
        })
    }
}

impl ToFieldFunction for &'static str {
    #[inline]
    fn to_field_function(self, protocol: Protocol) -> alloc::Result<AssociatedName> {
        Ok(AssociatedName {
            kind: meta::AssociatedKind::FieldFn(protocol, Cow::Borrowed(self)),
            function_parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: Vec::new(),
        })
    }
}

/// The full naming of an associated item.
pub struct Associated {
    /// The name of the associated item.
    pub(crate) name: AssociatedName,
    /// The container the associated item is associated with.
    pub(crate) container: FullTypeOf,
    /// Type info for the container the associated item is associated with.
    pub(crate) container_type_info: TypeInfo,
}

impl Associated {
    /// Construct a raw associated name.
    pub fn new(name: AssociatedName, container: FullTypeOf, container_type_info: TypeInfo) -> Self {
        Self {
            name,
            container,
            container_type_info,
        }
    }

    /// Construct an associated name from static type information.
    pub fn from_type<T>(name: AssociatedName) -> alloc::Result<Self>
    where
        T: TypeOf,
    {
        Ok(Self {
            name,
            container: T::type_of(),
            container_type_info: T::type_info(),
        })
    }

    /// Get unique key for the associated item.
    pub(crate) fn as_key(&self) -> alloc::Result<AssociatedKey> {
        Ok(AssociatedKey {
            type_hash: self.container.hash,
            kind: self.name.kind.try_clone()?,
            parameters: self.name.function_parameters,
        })
    }
}

/// Runtime data for an associated function.
pub struct AssociatedFunctionData {
    pub(crate) associated: Associated,
    pub(crate) handler: Arc<FunctionHandler>,
    #[cfg(feature = "doc")]
    pub(crate) is_async: bool,
    #[cfg(feature = "doc")]
    pub(crate) args: Option<usize>,
    #[cfg(feature = "doc")]
    pub(crate) return_type: Option<FullTypeOf>,
    #[cfg(feature = "doc")]
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
}

impl AssociatedFunctionData {
    pub(crate) fn from_raw(associated: Associated, handler: Arc<FunctionHandler>) -> Self {
        Self {
            associated,
            handler,
            #[cfg(feature = "doc")]
            is_async: false,
            #[cfg(feature = "doc")]
            args: None,
            #[cfg(feature = "doc")]
            return_type: None,
            #[cfg(feature = "doc")]
            argument_types: Box::default(),
        }
    }

    #[inline]
    pub(crate) fn from_function<F, A, K>(associated: Associated, f: F) -> alloc::Result<Self>
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Ok(Self {
            associated,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            #[cfg(feature = "doc")]
            is_async: K::is_async(),
            #[cfg(feature = "doc")]
            args: Some(F::args()),
            #[cfg(feature = "doc")]
            return_type: F::Return::maybe_type_of(),
            #[cfg(feature = "doc")]
            argument_types: A::into_box()?,
        })
    }

    #[inline]
    pub(crate) fn from_instance_function<F, A, K>(name: AssociatedName, f: F) -> alloc::Result<Self>
    where
        F: InstanceFunction<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Ok(Self {
            associated: Associated::from_type::<F::Instance>(name)?,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            #[cfg(feature = "doc")]
            is_async: K::is_async(),
            #[cfg(feature = "doc")]
            args: Some(F::args()),
            #[cfg(feature = "doc")]
            return_type: F::Return::maybe_type_of(),
            #[cfg(feature = "doc")]
            argument_types: A::into_box()?,
        })
    }
}

/// The kind of a [`FunctionMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[doc(hidden)]
pub enum FunctionMetaKind {
    #[doc(hidden)]
    Function(FunctionData),
    #[doc(hidden)]
    AssociatedFunction(AssociatedFunctionData),
}

impl FunctionMetaKind {
    #[doc(hidden)]
    #[inline]
    pub fn function<N, F, A, K>(name: N, f: F) -> alloc::Result<FunctionBuilder<N, F, A, K>>
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Ok(FunctionBuilder::new(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn instance<N, F, A, K>(name: N, f: F) -> alloc::Result<Self>
    where
        N: ToInstance,
        F: InstanceFunction<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Ok(Self::AssociatedFunction(
            AssociatedFunctionData::from_instance_function(name.to_instance()?, f)?,
        ))
    }
}

#[doc(hidden)]
pub struct FunctionBuilder<N, F, A, K> {
    name: N,
    f: F,
    _marker: PhantomData<(A, K)>,
}

impl<N, F, A, K> FunctionBuilder<N, F, A, K> {
    pub(crate) fn new(name: N, f: F) -> Self {
        Self {
            name,
            f,
            _marker: PhantomData,
        }
    }
}

impl<N, F, A, K> FunctionBuilder<N, F, A, K>
where
    F: Function<A, K>,
    F::Return: MaybeTypeOf,
    A: FunctionArgs,
    K: FunctionKind,
{
    #[doc(hidden)]
    #[inline]
    pub fn build(self) -> alloc::Result<FunctionMetaKind>
    where
        N: IntoComponent,
    {
        Ok(FunctionMetaKind::Function(FunctionData::new(
            self.name, self.f,
        )?))
    }

    #[doc(hidden)]
    #[inline]
    pub fn build_associated<T>(self) -> alloc::Result<FunctionMetaKind>
    where
        N: ToInstance,
        T: TypeOf,
    {
        let associated = Associated::from_type::<T>(self.name.to_instance()?)?;

        Ok(FunctionMetaKind::AssociatedFunction(
            AssociatedFunctionData::from_function(associated, self.f)?,
        ))
    }

    #[doc(hidden)]
    #[inline]
    pub fn build_associated_with(
        self,
        container: FullTypeOf,
        container_type_info: TypeInfo,
    ) -> alloc::Result<FunctionMetaKind>
    where
        N: ToInstance,
    {
        let name = self.name.to_instance()?;
        let associated = Associated::new(name, container, container_type_info);

        Ok(FunctionMetaKind::AssociatedFunction(
            AssociatedFunctionData::from_function(associated, self.f)?,
        ))
    }
}

/// The kind of a [`FunctionMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[doc(hidden)]
pub enum MacroMetaKind {
    #[doc(hidden)]
    Function(FunctionMacroData),
    #[doc(hidden)]
    Attribute(AttributeMacroData),
}

impl MacroMetaKind {
    #[doc(hidden)]
    #[inline]
    pub fn function<F, N>(name: N, f: F) -> alloc::Result<Self>
    where
        F: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_, '_, '_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Ok(Self::Function(FunctionMacroData::new(name, f)?))
    }

    #[doc(hidden)]
    #[inline]
    pub fn attribute<F, N>(name: N, f: F) -> alloc::Result<Self>
    where
        F: 'static
            + Send
            + Sync
            + Fn(
                &mut MacroContext<'_, '_, '_>,
                &TokenStream,
                &TokenStream,
            ) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Ok(Self::Attribute(AttributeMacroData::new(name, f)?))
    }
}

/// The data of a [`MacroMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[doc(hidden)]
pub struct MacroMetaData {
    #[doc(hidden)]
    pub kind: MacroMetaKind,
    #[doc(hidden)]
    pub name: &'static str,
    #[doc(hidden)]
    pub docs: &'static [&'static str],
}

/// The data of a [`FunctionMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[doc(hidden)]
pub struct FunctionMetaData {
    #[doc(hidden)]
    pub kind: FunctionMetaKind,
    #[doc(hidden)]
    pub name: &'static str,
    #[doc(hidden)]
    pub deprecated: Option<&'static str>,
    #[doc(hidden)]
    pub docs: &'static [&'static str],
    #[doc(hidden)]
    pub arguments: &'static [&'static str],
}

/// Trait implement allowing the collection of function argument types.
#[doc(hidden)]
pub trait FunctionArgs {
    #[doc(hidden)]
    fn into_box() -> alloc::Result<Box<[Option<FullTypeOf>]>>;
}

macro_rules! iter_function_args {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<$($ty,)*> FunctionArgs for ($($ty,)*)
        where
            $($ty: MaybeTypeOf,)*
        {
            #[inline]
            #[doc(hidden)]
            fn into_box() -> alloc::Result<Box<[Option<FullTypeOf>]>> {
                try_vec![$(<$ty>::maybe_type_of()),*].try_into_boxed_slice()
            }
        }
    }
}

repeat_macro!(iter_function_args);
