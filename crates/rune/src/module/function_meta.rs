use core::marker::PhantomData;

use crate::no_std::borrow::Cow;
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::{self, meta, IntoComponent, ItemBuf, Named};
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
pub type FunctionMeta = fn() -> FunctionMetaData;

/// Type used to collect and store function metadata through the
/// `#[rune::macro_]` macro.
///
/// This is the argument type for
/// [`Module::macro_meta`][crate::module::Module::macro_meta], and is from a
/// public API perspective completely opaque and might change for any release.
///
/// Calling and making use of `MacroMeta` manually despite this warning might
/// lead to future breakage.
pub type MacroMeta = fn() -> MacroMetaData;

/// Runtime data for a function.
#[derive(Clone)]
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
    #[inline]
    pub(crate) fn new<F, A, N, K>(name: N, f: F) -> Self
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        N: IntoIterator,
        N::Item: IntoComponent,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Self {
            item: ItemBuf::with_item(name),
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            #[cfg(feature = "doc")]
            is_async: K::is_async(),
            #[cfg(feature = "doc")]
            args: Some(F::args()),
            #[cfg(feature = "doc")]
            return_type: F::Return::maybe_type_of(),
            #[cfg(feature = "doc")]
            argument_types: A::into_box(),
        }
    }
}

/// Runtime data for a macro.
#[derive(Clone)]
pub struct FunctionMacroData {
    pub(crate) item: ItemBuf,
    pub(crate) handler: Arc<MacroHandler>,
}

impl FunctionMacroData {
    #[inline]
    pub(crate) fn new<F, N>(name: N, f: F) -> Self
    where
        F: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_, '_, '_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Self {
            item: ItemBuf::with_item(name),
            handler: Arc::new(f),
        }
    }
}

/// Runtime data for an attribute macro.
#[derive(Clone)]
pub struct AttributeMacroData {
    pub(crate) item: ItemBuf,
    pub(crate) handler: Arc<AttributeMacroHandler>,
}

impl AttributeMacroData {
    #[inline]
    pub(crate) fn new<F, N>(name: N, f: F) -> Self
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
        Self {
            item: ItemBuf::with_item(name),
            handler: Arc::new(f),
        }
    }
}

/// A descriptor for an instance function.
#[derive(Debug, Clone)]
#[non_exhaustive]
#[doc(hidden)]
pub struct AssociatedFunctionName {
    /// The name of the instance function.
    pub kind: meta::AssociatedKind,
    /// Parameters hash.
    pub function_parameters: Hash,
    #[cfg(feature = "doc")]
    pub parameter_types: Vec<Hash>,
}

impl AssociatedFunctionName {
    pub(crate) fn index(protocol: Protocol, index: usize) -> Self {
        Self {
            kind: meta::AssociatedKind::IndexFn(protocol, index),
            function_parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: vec![],
        }
    }
}

/// Trait used solely to construct an instance function.
pub trait ToInstance: self::sealed::Sealed {
    /// Get information on the naming of the instance function.
    #[doc(hidden)]
    fn to_instance(self) -> AssociatedFunctionName;
}

/// Trait used to determine what can be used as an instance function name.
pub trait ToFieldFunction: self::sealed::Sealed {
    #[doc(hidden)]
    fn to_field_function(self, protocol: Protocol) -> AssociatedFunctionName;
}

impl ToInstance for &'static str {
    #[inline]
    fn to_instance(self) -> AssociatedFunctionName {
        AssociatedFunctionName {
            kind: meta::AssociatedKind::Instance(Cow::Borrowed(self)),
            function_parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: vec![],
        }
    }
}

impl ToFieldFunction for &'static str {
    #[inline]
    fn to_field_function(self, protocol: Protocol) -> AssociatedFunctionName {
        AssociatedFunctionName {
            kind: meta::AssociatedKind::FieldFn(protocol, Cow::Borrowed(self)),
            function_parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: vec![],
        }
    }
}

/// Runtime data for an associated function.
#[derive(Clone)]
pub struct AssociatedFunctionData {
    pub(crate) name: AssociatedFunctionName,
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) container: FullTypeOf,
    pub(crate) container_type_info: TypeInfo,
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
    #[inline]
    pub(crate) fn new<F, A, K>(name: AssociatedFunctionName, f: F) -> Self
    where
        F: InstanceFunction<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Self {
            name,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            container: F::Instance::type_of(),
            container_type_info: F::Instance::type_info(),
            #[cfg(feature = "doc")]
            is_async: K::is_async(),
            #[cfg(feature = "doc")]
            args: Some(F::args()),
            #[cfg(feature = "doc")]
            return_type: F::Return::maybe_type_of(),
            #[cfg(feature = "doc")]
            argument_types: A::into_box(),
        }
    }

    /// Get associated key.
    pub(crate) fn assoc_key(&self) -> AssociatedKey {
        AssociatedKey {
            type_hash: self.container.hash,
            kind: self.name.kind.clone(),
            parameters: self.name.function_parameters,
        }
    }
}

/// The kind of a [`FunctionMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[derive(Clone)]
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
    pub fn function<N, F, A, K>(name: N, f: F) -> FunctionBuilder<N, F, A, K>
    where
        F: Function<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        FunctionBuilder {
            name,
            f,
            _marker: PhantomData,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn instance<N, F, A, K>(name: N, f: F) -> Self
    where
        N: ToInstance,
        F: InstanceFunction<A, K>,
        F::Return: MaybeTypeOf,
        A: FunctionArgs,
        K: FunctionKind,
    {
        Self::AssociatedFunction(AssociatedFunctionData::new(name.to_instance(), f))
    }
}

#[doc(hidden)]
pub struct FunctionBuilder<N, F, A, K> {
    name: N,
    f: F,
    _marker: PhantomData<(A, K)>,
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
    pub fn build(self) -> FunctionMetaKind
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        FunctionMetaKind::Function(FunctionData::new(self.name, self.f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn build_associated<T>(self) -> FunctionMetaKind
    where
        N: ToInstance,
        T: TypeOf + Named,
    {
        FunctionMetaKind::AssociatedFunction(AssociatedFunctionData {
            name: self.name.to_instance(),
            handler: Arc::new(move |stack, args| self.f.fn_call(stack, args)),
            container: T::type_of(),
            container_type_info: T::type_info(),
            #[cfg(feature = "doc")]
            is_async: K::is_async(),
            #[cfg(feature = "doc")]
            args: Some(F::args()),
            #[cfg(feature = "doc")]
            return_type: F::Return::maybe_type_of(),
            #[cfg(feature = "doc")]
            argument_types: A::into_box(),
        })
    }
}

/// The kind of a [`FunctionMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[derive(Clone)]
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
    pub fn function<F, N>(name: N, f: F) -> Self
    where
        F: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_, '_, '_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Self::Function(FunctionMacroData::new(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn attribute<F, N>(name: N, f: F) -> Self
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
        Self::Attribute(AttributeMacroData::new(name, f))
    }
}

/// The data of a [`MacroMeta`].
///
/// Even though this is marked as `pub`, this is private API. If you use this it
/// might cause breakage.
#[doc(hidden)]
#[derive(Clone)]
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
#[derive(Clone)]
pub struct FunctionMetaData {
    #[doc(hidden)]
    pub kind: FunctionMetaKind,
    #[doc(hidden)]
    pub name: &'static str,
    #[doc(hidden)]
    pub docs: &'static [&'static str],
    #[doc(hidden)]
    pub arguments: &'static [&'static str],
}

/// Trait implement allowing the collection of function argument types.
#[doc(hidden)]
pub trait FunctionArgs {
    #[doc(hidden)]
    fn into_box() -> Box<[Option<FullTypeOf>]>;
}

macro_rules! iter_function_args {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<$($ty,)*> FunctionArgs for ($($ty,)*)
        where
            $($ty: MaybeTypeOf,)*
        {
            #[inline]
            #[doc(hidden)]
            fn into_box() -> Box<[Option<FullTypeOf>]> {
                vec![$(<$ty>::maybe_type_of(),)*].into()
            }
        }
    }
}

repeat_macro!(iter_function_args);
