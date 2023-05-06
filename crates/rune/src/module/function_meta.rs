use core::fmt;
use core::future::Future;

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::{self, IntoComponent, ItemBuf, Named};
use crate::hash::Hash;
use crate::macros::{MacroContext, TokenStream};
use crate::module::{
    AssocType, AssociatedFunctionKey, AsyncFunction, AsyncInstFn, Function, InstFn,
};
use crate::runtime::{FullTypeOf, FunctionHandler, MacroHandler, MaybeTypeOf, Protocol};

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
    pub(crate) is_async: bool,
    pub(crate) name: ItemBuf,
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) args: Option<usize>,
    pub(crate) return_type: Option<FullTypeOf>,
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
}

impl FunctionData {
    #[inline]
    pub(crate) fn new<Func, Args, N>(name: N, f: Func) -> Self
    where
        Func: Function<Args>,
        Func::Return: MaybeTypeOf,
        N: IntoIterator,
        N::Item: IntoComponent,
        Args: IterFunctionArgs,
    {
        let mut argument_types = Vec::with_capacity(Args::len());
        Args::iter_args(|ty| argument_types.push(ty));

        Self {
            is_async: false,
            name: ItemBuf::with_item(name),
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            args: Some(Func::args()),
            return_type: Func::Return::maybe_type_of(),
            argument_types: argument_types.into(),
        }
    }

    #[inline]
    pub(crate) fn new_async<Func, Args, N>(name: N, f: Func) -> Self
    where
        Func: AsyncFunction<Args>,
        Func::Output: MaybeTypeOf,
        N: IntoIterator,
        N::Item: IntoComponent,
        Args: IterFunctionArgs,
    {
        let mut argument_types = Vec::with_capacity(Args::len());
        Args::iter_args(|ty| argument_types.push(ty));

        Self {
            is_async: true,
            name: ItemBuf::with_item(name),
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            args: Some(Func::args()),
            return_type: Func::Output::maybe_type_of(),
            argument_types: argument_types.into(),
        }
    }
}

/// Runtime data for a macro.
#[derive(Clone)]
pub struct FunctionMacroData {
    pub(crate) name: ItemBuf,
    pub(crate) handler: Arc<MacroHandler>,
}

impl FunctionMacroData {
    #[inline]
    pub(crate) fn new<Func, N>(name: N, f: Func) -> Self
    where
        Func: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Self {
            name: ItemBuf::with_item(name),
            handler: Arc::new(f),
        }
    }
}

/// An instance function name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AssociatedFunctionKind {
    /// A protocol function implemented on the type itself.
    Protocol(Protocol),
    /// A field function with the given protocol.
    FieldFn(Protocol, Box<str>),
    /// An index function with the given protocol.
    IndexFn(Protocol, usize),
    /// The instance function refers to the given named instance fn.
    Instance(Box<str>),
}

impl AssociatedFunctionKind {
    /// Convert the kind into a hash function.
    pub(crate) fn hash(&self, instance_type: Hash) -> Hash {
        match self {
            Self::Protocol(protocol) => Hash::instance_function(instance_type, protocol.hash),
            Self::IndexFn(protocol, index) => {
                Hash::index_fn(*protocol, instance_type, Hash::index(*index))
            }
            Self::FieldFn(protocol, field) => {
                Hash::field_fn(*protocol, instance_type, field.as_ref())
            }
            Self::Instance(name) => Hash::instance_function(instance_type, name.as_ref()),
        }
    }
}

impl fmt::Display for AssociatedFunctionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssociatedFunctionKind::Protocol(protocol) => write!(f, "<{}>", protocol.name),
            AssociatedFunctionKind::FieldFn(protocol, field) => {
                write!(f, ".{field}<{}>", protocol.name)
            }
            AssociatedFunctionKind::IndexFn(protocol, index) => {
                write!(f, ".{index}<{}>", protocol.name)
            }
            AssociatedFunctionKind::Instance(name) => write!(f, "{}", name),
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

impl ToInstance for &str {
    #[inline]
    fn to_instance(self) -> AssociatedFunctionName {
        AssociatedFunctionName {
            kind: AssociatedFunctionKind::Instance(self.into()),
            parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: vec![],
        }
    }
}

impl ToFieldFunction for &str {
    #[inline]
    fn to_field_function(self, protocol: Protocol) -> AssociatedFunctionName {
        AssociatedFunctionName {
            kind: AssociatedFunctionKind::FieldFn(protocol, self.into()),
            parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: vec![],
        }
    }
}

/// A descriptor for an instance function.
#[derive(Clone)]
#[non_exhaustive]
#[doc(hidden)]
pub struct AssociatedFunctionName {
    /// The name of the instance function.
    pub kind: AssociatedFunctionKind,
    /// Parameters hash.
    pub parameters: Hash,
    #[cfg(feature = "doc")]
    pub parameter_types: Vec<Hash>,
}

impl AssociatedFunctionName {
    pub(crate) fn index(protocol: Protocol, index: usize) -> Self {
        Self {
            kind: AssociatedFunctionKind::IndexFn(protocol, index),
            parameters: Hash::EMPTY,
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
    pub(crate) ty: AssocType,
    pub(crate) is_async: bool,
    pub(crate) args: Option<usize>,
    pub(crate) return_type: Option<FullTypeOf>,
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
}

impl AssociatedFunctionData {
    #[inline]
    pub(crate) fn new<Func, Args>(name: AssociatedFunctionName, f: Func) -> Self
    where
        Func: InstFn<Args>,
        Func::Return: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        let mut argument_types = Vec::with_capacity(Args::len());
        Args::iter_args(|ty| argument_types.push(ty));

        Self {
            name,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            ty: Func::ty(),
            is_async: false,
            args: Some(Func::args()),
            return_type: Func::Return::maybe_type_of(),
            argument_types: argument_types.into(),
        }
    }

    #[inline]
    pub(crate) fn new_async<Func, Args>(name: AssociatedFunctionName, f: Func) -> Self
    where
        Func: AsyncInstFn<Args>,
        Func::Output: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        let mut argument_types = Vec::with_capacity(Args::len());
        Args::iter_args(|ty| argument_types.push(ty));

        Self {
            name,
            handler: Arc::new(move |stack, args| f.fn_call(stack, args)),
            ty: Func::ty(),
            is_async: true,
            args: Some(Func::args()),
            return_type: <Func::Return as Future>::Output::maybe_type_of(),
            argument_types: argument_types.into(),
        }
    }

    /// Get associated key.
    pub(crate) fn assoc_key(&self) -> AssociatedFunctionKey {
        AssociatedFunctionKey {
            type_hash: self.ty.hash,
            kind: self.name.kind.clone(),
            parameters: self.name.parameters,
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
    pub fn function<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: Function<Args>,
        Func::Return: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        Self::Function(FunctionData::new(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn function_with<T, N, Func, Args>(name: N, f: Func) -> Self
    where
        T: Named,
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: Function<Args>,
        Func::Return: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        let name = [IntoComponent::into_component(T::BASE_NAME)]
            .into_iter()
            .chain(name.into_iter().map(IntoComponent::into_component));
        Self::Function(FunctionData::new(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn async_function<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: AsyncFunction<Args>,
        Func::Output: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        Self::Function(FunctionData::new_async(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn async_function_with<T, N, Func, Args>(name: N, f: Func) -> Self
    where
        T: Named,
        N: IntoIterator,
        N::Item: IntoComponent,
        Func: AsyncFunction<Args>,
        Func::Output: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        let name = [IntoComponent::into_component(T::BASE_NAME)]
            .into_iter()
            .chain(name.into_iter().map(IntoComponent::into_component));
        Self::Function(FunctionData::new_async(name, f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn instance<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: ToInstance,
        Func: InstFn<Args>,
        Func::Return: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        Self::AssociatedFunction(AssociatedFunctionData::new(name.to_instance(), f))
    }

    #[doc(hidden)]
    #[inline]
    pub fn async_instance<N, Func, Args>(name: N, f: Func) -> Self
    where
        N: ToInstance,
        Func: AsyncInstFn<Args>,
        Func::Output: MaybeTypeOf,
        Args: IterFunctionArgs,
    {
        Self::AssociatedFunction(AssociatedFunctionData::new_async(name.to_instance(), f))
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
}

impl MacroMetaKind {
    #[doc(hidden)]
    #[inline]
    pub fn function<Func, N>(name: N, f: Func) -> Self
    where
        Func: 'static
            + Send
            + Sync
            + Fn(&mut MacroContext<'_>, &TokenStream) -> compile::Result<TokenStream>,
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        Self::Function(FunctionMacroData::new(name, f))
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
pub trait IterFunctionArgs {
    /// The number of arguments being passed in.
    fn len() -> usize;

    /// Iterate over arguments providing their full type information in the
    /// process.
    #[doc(hidden)]
    fn iter_args<Receiver>(receiver: Receiver)
    where
        Receiver: FnMut(Option<FullTypeOf>);
}

macro_rules! iter_function_args {
    ($count:expr $(, $ty:ident $var:ident $num:expr)*) => {
        impl<$($ty,)*> IterFunctionArgs for ($($ty,)*)
        where
            $($ty: MaybeTypeOf,)*
        {
            #[inline]
            fn len() -> usize {
                $count
            }

            #[inline]
            fn iter_args<Receiver>(#[allow(unused)] mut receiver: Receiver) where Receiver: FnMut(Option<FullTypeOf>) {
                $(receiver(<$ty>::maybe_type_of());)*
            }
        }
    }
}

repeat_macro!(iter_function_args);
